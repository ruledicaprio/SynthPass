- **An iPhone photo now says what it is instead of looking like a broken page.** `createImageBitmap`
  rejects HEIC/HEIF in most browser engines — and HEIC is the iPhone default — with a bare
  `The source image could not be decoded`, which reads as a corrupt file rather than an unsupported
  format. The demo surfaced that verbatim as `Failed: The source image could not be decoded`, giving
  a user no idea that re-saving as JPEG would fix it. HEIC and PDF are now named, with the fix
  spelled out.

  The bytes decide, not the file name: an iPhone photo shared through a chat app routinely arrives
  with the wrong extension, which is exactly the case a name-based check misses — the same lesson as
  the native decoder fix in #192. It is literally the same magic-byte table, reached through
  `mrz-wasm` rather than written a second time in JavaScript, so the browser and
  `synthpass_ocr::decode_image` cannot drift on what a file is.

  Unrecognised bytes still get **no invented diagnosis** — a wrong explanation sends the reader after
  the wrong problem — but the fallback message at least names the formats that do work. An MP4 is
  ISO-BMFF like HEIC is, and is deliberately *not* reported as HEIC; there is a test for that.

- **The file pickers accept HEIC.** They previously listed only `.jpg,.jpeg,.png,.webp,.bmp`, so on
  a phone the format most likely to be selected was the one the picker hid — leaving the user unable
  to select the file and no wiser as to why. Selecting one now produces the actionable message above.

- **`tests/web/check-format-errors.mjs`** verifies all of this end to end in real Chromium, through
  the shipped `scanDocument` path: each undecodable format's message, the absence of a diagnosis for
  bytes we do not recognise, and — so the negative cases mean something — that a real JPEG still
  decodes.
