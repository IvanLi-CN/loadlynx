# loadlynx-calibration-format

Pure, host-testable calibration logic shared by the ESP32-S3 firmware:

- CalWrite multi-chunk payload encoding (little-endian) + inner CRC16 (CCITT-FALSE over `index + payload`).
- EEPROM calibration profile serialization/deserialization + CRC32 (IEEE).

