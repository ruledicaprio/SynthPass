- **Damaged-zone recovery no longer invents the format letter.** A passport whose line 1 OCR
  dropped a character could be returned as a checksum-valid MRV-A visa: restoration inserted
  `V` at cell 0, and the visa layout lacks TD3's personal-number and composite checks.
  Restoration now keeps the observed first cell while still recovering missing interior cells.
