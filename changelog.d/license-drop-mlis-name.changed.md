- **Licence files use the SynthPass name.** The default licence path is `license.synthpass`
  (was `license.mlis`) in `synthpass`, `synthpass-serve` and `synthpass-license-issuer`. The
  payload field is `synthpass_min_version` (was `mlis_min_version`), and the Linux fingerprint
  fallback id lives at `/var/lib/synthpass/instance-id` (was `/var/lib/mlis/instance-id`).
  `SYNTHPASS_LICENSE_PATH` and `SYNTHPASS_INSTANCE_ID_PATH` still override both paths. No
  licence has been issued (the verifying key is still a placeholder), so no existing file is
  affected.
