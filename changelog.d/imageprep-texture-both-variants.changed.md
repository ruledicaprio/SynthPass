The texture-suppression stage now appends two trailing variants rather than one: the untreated
band crop followed by the median-filtered one. Measuring them separately showed they recover
different kinds of miss — the untreated crop rescues documents whose MRZ was never located, the
median rescues documents whose MRZ was located and misread — so running only one of them meant
each arm lost the documents the other would have caught. Both still run last, after every
variant that succeeds today, so neither can cost a document that already reads.
