#![no_std]
#![no_main]

use nanos_sdk::buttons::ButtonEvent;
use nanos_sdk::ecc::{Secp256k1, SeedDerive};
use nanos_sdk::io;
use nanos_sdk::io::ApduHeader;
use nanos_sdk::io::Reply;

use nanos_ui::{ui::Validator, bitmaps::Glyph};
use nanos_ui::layout::{Layout, Location, StringPlace};

use include_gif::include_gif;

nanos_sdk::set_panic!(nanos_sdk::exiting_panic);

pub const AGE_BIP32_PATH: [u32; 4] = nanos_sdk::ecc::make_bip32_path(b"m/414745'/0'/0'/0'");
pub const LOGO: Glyph = Glyph::from_include(include_gif!("logo.gif"));

fn display_homescreen() {
    LOGO.invert().draw(0, 0);
    "Ledger ready".place(Location::Bottom, Layout::RightAligned, false);
}

#[no_mangle]
extern "C" fn sample_main() {
    let mut comm = io::Comm::new();

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
            },
            io::Event::Ticker => {
                if standby_tick_count == 0 {
                    // Show message
                    display_homescreen();
                } else if standby_tick_count >= 300 {
                    nanos_sdk::exit_app(0);
                }

                standby_tick_count += 1;
            },
            _ => (),
        }
    }
}

#[repr(u8)]
enum Ins {
    GetRecipient,
    Unwrap,
}

impl From<ApduHeader> for Ins {
    fn from(header: ApduHeader) -> Ins {
        match header.ins {
            2 => Ins::GetRecipient,
            3 => Ins::Unwrap,
            _ => panic!(),
        }
    }
}


fn handle_apdu(comm: &mut io::Comm, ins: Ins) -> Result<(), Reply> {
    if comm.rx == 0 {
        return Err(io::StatusWords::NothingReceived.into());
    }

    match ins {
        Ins::GetRecipient => {
            let pk = Secp256k1::derive_from_path(&AGE_BIP32_PATH)
                .public_key()
                .map_err(|x| Reply(0x6eu16 | (x as u16 & 0xff)))?;
            comm.append(pk.as_ref());
        }
        Ins::Unwrap => {
            if Validator::new("Decrypt message").ask() {
                let sk = Secp256k1::derive_from_path(&AGE_BIP32_PATH);
                let share = comm.get_data()?;
                let ans = sk.ecdh(share)
                    .map_err(|_| Reply(0x6f00u16))?;
                comm.append(ans.as_ref());
            }
        }
    }

    display_homescreen();

    Ok(())
}
