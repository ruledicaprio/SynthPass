- **`MrzData::personal_number` is gone: `optional_data_1` and `optional_data_2` replace it, and
  `personal_number()` names the TD3 element** (ADR-0018). The field held three different things
  under one name — TD3's personal number, TD2/MRV-A/MRV-B optional data, and TD1's two
  optional-data elements *joined with a space*, a join that could not be undone: a TD1 whose
  value sat in slot 1 was indistinguishable from one whose value sat in slot 2, and two tracked
  specimens (Belgium 2021, Serbia 2008) sit on opposite sides of exactly that. Now
  `optional_data_1` is the format's primary optional-data element — TD1 line 1 `[15,30)`, TD2
  `[28,35)`, TD3 `[28,42)`, MRV-A `[28,44)`, MRV-B `[28,36)` — and the document-number overflow
  target on every format that defines one; `optional_data_2` is TD1's second element (line 2
  `[18,29)`) and `None` on every other format; `personal_number()` returns `optional_data_1` on
  TD3 and `None` elsewhere, because only TD3 prints a personal number (Doc 9303 Part 4 §4.2.2
  titles that field "personal number or other optional data elements"). The parser now reports
  the shape the emitters (`Td1Fields`, `Td2Fields`, `MrvAFields`, `MrvBFields`, `Td3Fields`) have
  always taken. `Checks::personal_number` and `Field::PersonalNumber` are unchanged: they name
  the check digit, which only TD3 prints. The `serde` shape follows the fields — `personal_number`
  is no longer a key, `optional_data_1` and `optional_data_2` are. Migration: TD3 readers call
  `personal_number()` or read `optional_data_1`; TD2 and visa readers read `optional_data_1`;
  TD1 readers read both slots.
