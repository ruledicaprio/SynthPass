- **PDF lane in `tools/screen_candidates.py`.** A candidate whose download is a PDF (a gazette
  annex, a regulation, a guide -- 12 of the loop's first 66 none-found reports were exactly
  these) is no longer an off-scope reject: its pages are scanned for the specimen words the page
  text is scanned for, the images on those pages are extracted at their original bytes (a page
  render only when a page embeds none), and each image is screened as its own candidate with
  `#page=N` on its URL and a `pdf_source` record. PyMuPDF is the one non-standard-library
  dependency and is imported only by this lane. `tools/scout_cycle.py add-candidates` appends
  candidates the maintainer's session found itself (a retry of a worker's blocked pages) to a
  cycle before screening; the retry never runs from repository code.
