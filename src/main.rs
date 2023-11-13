#![no_std]
#![no_main]

use ledger_sdk_sys::{cx_hash_sha256, CX_SHA256_SIZE};
use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::ecc::{CxError, Secp256k1, SeedDerive};
use nanos_sdk::io;
use nanos_sdk::io::ApduHeader;
use nanos_sdk::io::Reply;

use nanos_ui::layout::{Layout, Location, StringPlace};
use nanos_ui::{
    bitmaps::Glyph,
    screen_util::screen_update,
    ui::{clear_screen, Validator},
};

use include_gif::include_gif;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

pub const AGE_BIP32_PATH: [u32; 3] = nanos_sdk::ecc::make_bip32_path(b"m/6383461'/0'/0");
pub const LOGO: Glyph = Glyph::from_include(include_gif!("logo.gif"));

fn display_homescreen() {
    LOGO.invert().draw(0, 0);
    "Ledger ready".place(Location::Bottom, Layout::RightAligned, false);
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

    // Developer mode / pending review popup
    // must be cleared with user interaction
    {
        use ButtonEvent::*;

        clear_screen();
        "Pending Review".place(Location::Middle, Layout::Centered, false);
        screen_update();

        loop {
            match comm.next_event::<ApduHeader>() {
                io::Event::Button(LeftButtonRelease | RightButtonRelease | BothButtonsRelease) => {
                    break
                }
                _ => (),
            }
        }
    }

    // Increased every tick until standby. Resetted if a button is pressed.
    let mut standby_tick_count = 0;

    loop {
        // Wait for either a specific button push to exit the app
        // or an APDU command
        match comm.next_event() {
            io::Event::Button(ButtonEvent::BothButtonsRelease) => nanos_sdk::exit_app(0),
            io::Event::Command(ins) => {
                match handle_apdu(&mut comm, ins) {
                    Ok(()) => comm.reply_ok(),
                    Err(sw) => comm.reply(sw),
                };
                standby_tick_count = 0;
            }
            io::Event::Ticker => {
                if standby_tick_count == 0 {
                    // Show message
                    display_homescreen();
                } else if standby_tick_count >= 3000 {
                    nanos_sdk::exit_app(0);
                }

                standby_tick_count += 1;
            }
            _ => (),
        }
    }
}

#[repr(u8)]
enum Ins {
    GetRecipient,
    ConfirmRecipient,
    GetSharedKey,
}

impl From<ApduHeader> for Ins {
    fn from(header: ApduHeader) -> Ins {
        match header.ins {
            1 => Ins::ConfirmRecipient,
            2 => Ins::GetRecipient,
            3 => Ins::GetSharedKey,
            _ => panic!(),
        }
    }
}

enum AgeError {
    CryptoError(u16),
    TagMismatch,
    DataError,
    DeniedByUser,
}

impl From<CxError> for AgeError {
    fn from(code: CxError) -> AgeError {
        AgeError::CryptoError(code as u16)
    }
}

impl From<io::StatusWords> for AgeError {
    fn from(_: io::StatusWords) -> AgeError {
        AgeError::DataError
    }
}
impl From<AgeError> for Reply {
    fn from(c: AgeError) -> Reply {
        match c {
            AgeError::CryptoError(code) => Reply(code),
            AgeError::TagMismatch => Reply(0x6af0),
            AgeError::DataError => Reply(0x6e77),
            AgeError::DeniedByUser => Reply(0x69f0),
        }
    }
}

fn handle_apdu(comm: &mut io::Comm, ins: Ins) -> Result<(), AgeError> {
    match ins {
        Ins::ConfirmRecipient => {
            let tag = comm.get_data()?;
            let pk = Secp256k1::derive_from_path(&AGE_BIP32_PATH).public_key()?;
            let mut recomputed_tag = [0; CX_SHA256_SIZE as usize];

            unsafe {
                cx_hash_sha256(
                    pk.as_ref().as_ptr(),
                    pk.as_ref().len(),
                    recomputed_tag.as_mut_ptr(),
                    recomputed_tag.len(),
                );
            }

            if tag != recomputed_tag {
                return Err(AgeError::TagMismatch);
            }

            comm.append(pk.as_ref());
        }
        Ins::GetRecipient => {
            if !Validator::new("Send recipient").ask() {
                return Err(AgeError::DeniedByUser);
            }
            let pk = Secp256k1::derive_from_path(&AGE_BIP32_PATH).public_key()?;
            comm.append(pk.as_ref());
        }
        Ins::GetSharedKey => {
            if !Validator::new("Send decryption key").ask() {
                return Err(AgeError::DeniedByUser);
            }
            let share = comm.get_data()?;
            let ans = Secp256k1::derive_from_path(&AGE_BIP32_PATH).ecdh(share)?;
            comm.append(ans.as_ref());
        }
    }

    display_homescreen();

    Ok(())
}
