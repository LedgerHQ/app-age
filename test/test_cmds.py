import pytest
import os
from binascii import hexlify, unhexlify

from ecpy.curves import Curve, Point
from Crypto.Protocol.KDF import HKDF
from Crypto.Hash import SHA256
from chacha20poly1305 import ChaCha20Poly1305
from bech32 import bech32_decode, convertbits
from base64 import b32decode
from hashlib import sha256

from ledgerwallet.client import LedgerClient

ABANDON_RECIPIENT = bytes.fromhex("04c6b1fec4cbbb3a3b1ff505959b4e2164760dd5b309139f2e7e60bde118969d07986c1829490c6a0d0b614816e22f204572a97708465e38b1a2a49fa3133c80c2")
hrp, tag_bits = bech32_decode("AGE-PLUGIN-LEDGER-1WH90T6PD06QN907ADRARJTDMR20K7K8HQQVY8RX7PSVGS5P3Q7PQLHDJXV")
ABANDON_TAG = bytes(convertbits(tag_bits, 5, 8, False))
assert len(ABANDON_TAG) == 32

d = LedgerClient()

def exchange_and_expect(expected_output_hex: str, ins: int, data_hex: str = ''):
    r = d.apdu_exchange(ins, bytes.fromhex(data_hex))
    print(f"<- {r.hex()}")
    assert r.hex() == expected_output_hex.lower()


def test_get_recipient():
    expected = ABANDON_RECIPIENT.hex()

    exchange_and_expect(expected, 0x02)

def test_get_shared_key():
    cv = Curve.get_curve('secp256k1')

    P = Point(int.from_bytes(ABANDON_RECIPIENT[1:33], "big"), int.from_bytes(ABANDON_RECIPIENT[33:65], "big"), cv)

    file_key = os.urandom(16)
    secret = int.from_bytes(os.urandom(32), "big")

    basepoint = Point(0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798, 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8, cv)
    ephemeral_share = bytes(cv.encode_point(secret * basepoint))

    salt = ephemeral_share + ABANDON_RECIPIENT
    info = b"ledger"
    shared_secret = bytes(cv.encode_point(secret * P))

    wrap_key = HKDF(shared_secret, 32, salt, SHA256, 1, info)
    cipher = ChaCha20Poly1305(wrap_key)
    body = cipher.encrypt(b'\0'*12, file_key)

    exchange_and_expect(shared_secret[1:33].hex(), 0x03, ephemeral_share.hex())

def test_confirm_recipient():
    exchange_and_expect(ABANDON_RECIPIENT.hex(), 0x01, ABANDON_TAG.hex())
