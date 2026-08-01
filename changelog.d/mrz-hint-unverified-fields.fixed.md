- **The Tier-2 MRZ hint no longer presents `nationality`/`sex` as checksum-verified.**
  `synthpass_pipeline::mrz_hint` emitted those two fields whenever `checks.composite` passed,
  under a prompt banner reading "Verified from the machine-readable zone (trust these over the
  OCR text)". No composite check digit covers either field in any format — TD1, TD2 and TD3 all
  exclude the `nationality` and `sex` positions from the composite range — and MRV-A/MRV-B have
  no composite check digit at all, reporting `checks.composite = true` vacuously, so on a visa
  the gate treated *no evidence* as proof. The hint now carries only the four individually
  check-digited fields, matching the gates `promote_verified_mrz_fields` already applies. Affects
  only runs with `SYNTHPASS_LLM_MRZ_HINT=1` (opt-in, off by default). The prompt template text is
  unchanged, so `PROMPT_VERSION` and the pinned prompt digest stay as they were.
