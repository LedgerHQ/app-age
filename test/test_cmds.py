import pytest
import os
from binascii import hexlify, unhexlify

from ecpy.curves import Curve, Point
from Crypto.Protocol.KDF import HKDF
from Crypto.Hash import SHA256
from chacha20poly1305 import ChaCha20Poly1305

from ledgerwallet.client import LedgerClient


d = LedgerClient()

def exchange_and_expect(expected_output_hex: str, ins: int, data_hex: str = ''):
    r = d.apdu_exchange(ins, bytes.fromhex(data_hex))
    print(f"<- {r.hex()}")
    assert r.hex() == expected_output_hex.lower()


def test_get_recipient():
    expected = "04fb52b66f3256a5c28dbfcee45a78ff984d597473801360da90adae60c986b851d59103c870c70d3b26bc5d85424e4ff0420ba215a04fa3b0b51598cbdcec600f"

    exchange_and_expect(expected, 0x02)

def test_unwrap():
    cv = Curve.get_curve('secp256k1')

    recipient = d.apdu_exchange(0x2)
    P = Point(int.from_bytes(recipient[1:33], "big"), int.from_bytes(recipient[33:65], "big"), cv)

    file_key = os.urandom(16)
    secret = int.from_bytes(os.urandom(32), "big")

    basepoint = Point(0x79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798, 0x483ada7726a3c4655da4fbfc0e1108a8fd17b448a68554199c47d08ffb10d4b8, cv)
    ephemeral_share = bytes(cv.encode_point(secret * basepoint))

    salt = ephemeral_share + recipient
    info = b"ledger"
    shared_secret = bytes(cv.encode_point(secret * P))

    wrap_key = HKDF(shared_secret, 32, salt, SHA256, 1, info)
    cipher = ChaCha20Poly1305(wrap_key)
    body = cipher.encrypt(b'\0'*12, file_key)

    exchange_and_expect(shared_secret[1:33].hex(), 0x03, ephemeral_share.hex())


#test_get_recipient()
#test_unwrap()
