# CrustyDB

Page - 4 KB ~ 4096 bytes
page metadata - 16 bytes: pagid(4byt + LSN(4+2 bytes page_id, slot_id) + 2byte for checksum, 4 bytes free to use)
heap metadata - 4 8 bytes


No record is larger than a block. and each record is contained in a single block