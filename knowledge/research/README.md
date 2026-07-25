# research/

Distilled, actionable summaries drawn from `../papers/`.

## The 80/20 filter

Reading a paper, ask one question: **can this become a trait, struct, algorithm,
pipeline stage, benchmark, configuration knob, or heuristic?** If not, it does not
get a note.

The value is extracting *design principles*, not *definitions*. A summary that
restates the abstract is worse than nothing — it costs context and teaches
nothing. Twenty pages of actionable extract beats four hundred and fifty pages of
source.

## What a research note contains

- The one idea worth stealing, stated in terms of *this* pipeline.
- What it would replace or improve, named by file and type.
- What it would cost — dependencies, latency, memory, complexity.
- Why it might not work here. Most won't; say so and keep the note anyway, so the
  next person doesn't re-derive the same dead end.

## Current source material

`../papers/` holds five open-set-recognition papers. OSR is relevant because a
document pipeline must be able to say *"I don't know"* rather than force a read
into the nearest known class — the same posture as
`fusion::Verdict::NeedsReview`. Of the five, the closest to actionable are the
unified-OSR uncertainty-fusion result and the gradient-based unknown detector.

None have been distilled yet.
