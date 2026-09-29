- **`synthpass-bench` counts a wrong document code or issuer on every accepted read, not only on
  hits.** A read that passes its check digits is accepted even when its document number differs
  from truth (a `document_number_mismatch` miss), so the hits-only prefix count missed it. The
  report gains `accepted_reads`, `prefix_wrong_accepted_reads` and
  `prefix_wrong_accepted_read_seeds`, each `results[]` entry gains `prefix_wrong_accepted_read`,
  and the terminal summary prints one more line. Report-only: `--max-prefix-wrong-accepts`, `hit`,
  `wrong_accept` and every existing field are unchanged (#574).
