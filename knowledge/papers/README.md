# papers/

Source academic material, kept as reference. Distilled findings go in
`../research/` — this folder is the raw input, not the output.

## Contents

Five of the six are open-set recognition (OSR). OSR matters here because a
document pipeline must be able to reject an input rather than force it into the
nearest known class — the same posture as `fusion::Verdict::NeedsReview`.

| File | What it argues |
|---|---|
| `Towards_Unified_OSR.md` | ICLR 2023. Misclassified-known samples produce near-identical uncertainty scores to genuine out-of-distribution samples, despite being feature-similar to correctly-classified ones. Proposes fusing a softmax uncertainty with a few-shot KNN score. **The most relevant of the five** — it is about uncertainty *fusion*, which is what a confidence engine does. |
| `Open_Set_Recognition_with_Gradient_Based_Representations.md` | ICIP 2021. Detects unknowns from *gradient magnitudes* w.r.t. a confounding label, rather than from learned features. |
| `Class_Specific_Semantic_Reconstruction_for_OSR.md` | CSSR: per-class autoencoders reconstructing *semantic* features rather than pixels, avoiding the classification degradation of earlier autoencoder approaches. |
| `Classification_Reconstruction_Learning_for_OSR.md` | CVPR 2019 (CROSR). Joint classification + reconstruction; rejection distance computed on concatenated prediction and latent representations. |
| `Understanding_OSR_by_Jacobian_Norm_and_Inter_Class_Separation.md` | Pattern Recognition 2023. Theory for *why* closed-set training alone yields some OSR capability. |
| `Counting_Objects_by_Spatial_Divide-and-Conquer.md` | S-DCNet (ICCV 2019). Not OSR classification — crowd counting — but the *same* structural move: an unbounded open-set quantity is made tractable by recursively subdividing until every sub-region falls inside the closed set the model was trained on, then summing. Read it for the decomposition strategy, not the counting. |

`figures/` holds figures extracted from the papers, prefixed with the filename
of the paper they came from. (It held only the ICLR paper's figures under the
name `figures_TOWARDS_UNIFIED_OSR/` until a second paper needed one.)

Two files carry `<img src="./figures/...">` references to plots that were never
extracted — `Understanding_OSR_by_Jacobian_Norm_and_Inter_Class_Separation.md`
is the main offender. Those links are broken and were broken before this tree
was reorganised; the prose stands without them.

## Note on provenance

`Class_Specific_Semantic_Reconstruction_for_OSR.md` was filed under a filename
from an unrelated crowd-counting paper; renamed to match its actual contents when
this tree was first committed. Check that a file's title matches its body before
citing it. (The crowd-counting paper that caused that confusion is now genuinely
here, under its own name — verified against its body, not just its title.)

Files arriving from a conversion step sometimes carry a preamble the converter
wrote about itself. Strip it. `Counting_Objects_by_Spatial_Divide-and-Conquer.md`
arrived with two paragraphs of "here is the cleaned, structured document" on top
of the actual paper.

## What does not belong here

PDFs. The point of this tree is that it is greppable, diffable text a model can
read in context. Convert on the way in.
