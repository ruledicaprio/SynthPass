- **The commercial model is MIT software plus paid services, not a feature-gated tier.**
  `BRANDING.md` §5 described a Community/Professional/Enterprise split whose boundary was
  "enforced through the existing offline Ed25519 licensing mechanism". That does not survive
  contact with this repository: the licence check is bypassable by recompiling (its own threat
  model in `ARCHITECTURE.md` §6 says it is metering, not DRM), the bypass is documented in the
  README quickstart as `SYNTHPASS_LICENSE_SKIP=1` because local development needs it, and
  `crates/synthpass-license/pubkey.b64` is still the placeholder its own comment says to replace
  before issuing anything real — so no licence has ever been issuable against a shipped binary.

  Gating a feature people can un-gate in an afternoon buys no revenue and costs the project its
  central claim: that you can read every line that touches your documents. The rewrite names
  what is actually sellable — labelled corpora generated to a customer's document mix,
  independent benchmarking and certification, air-gapped integration, custom-trained models,
  and support — and why none of it can be had by recompiling. `synthpass-license` stays, as
  honest capacity metering and entitlement records rather than as a wall.

  `VISION.md` §3, `knowledge/README.md`'s index entry and `ROADMAP.md`'s final M6 item follow.
  The sequencing consequence is the point: the generation and benchmarking side is sellable
  today at the current extraction accuracy, so it does not wait on the detection track.
