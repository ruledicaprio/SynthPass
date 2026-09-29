- **Whitespace-split OCR lines.** `find_and_parse` rejoins adjacent OCR tokens only when their
  combined width is exactly 30, 36 or 44 cells. The additive repair adds no refusal and changes no
  public API.
