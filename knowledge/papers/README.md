# papers/

Source academic material, kept as reference. Distilled findings go in
`../research/` — this folder is the raw input, not the output.

## Contents

All five are open-set recognition (OSR). OSR matters here because a document
pipeline must be able to reject an input rather than force it into the nearest
known class — the same posture as `fusion::Verdict::NeedsReview`.

| File | What it argues |
|---|---|
| `Towards_Unified_OSR.md` | ICLR 2023. Misclassified-known samples produce near-identical uncertainty scores to genuine out-of-distribution samples, despite being feature-similar to correctly-classified ones. Proposes fusing a softmax uncertainty with a few-shot KNN score. **The most relevant of the five** — it is about uncertainty *fusion*, which is what a confidence engine does. |
| `Open_Set_Recognition_with_Gradient_Based_Representations.md` | ICIP 2021. Detects unknowns from *gradient magnitudes* w.r.t. a confounding label, rather than from learned features. |
| `Class_Specific_Semantic_Reconstruction_for_OSR.md` | CSSR: per-class autoencoders reconstructing *semantic* features rather than pixels, avoiding the classification degradation of earlier autoencoder approaches. |
| `Classification_Reconstruction_Learning_for_OSR.md` | CVPR 2019 (CROSR). Joint classification + reconstruction; rejection distance computed on concatenated prediction and latent representations. |
| `Understanding_OSR_by_Jacobian_Norm_and_Inter_Class_Separation.md` | Pattern Recognition 2023. Theory for *why* closed-set training alone yields some OSR capability. |

`figures_TOWARDS_UNIFIED_OSR/` holds figures extracted from the ICLR paper.

## Note on provenance

`Class_Specific_Semantic_Reconstruction_for_OSR.md` was filed under a filename
from an unrelated crowd-counting paper; renamed to match its actual contents when
this tree was first committed. Check that a file's title matches its body before
citing it.

## What does not belong here

PDFs. The point of this tree is that it is greppable, diffable text a model can
read in context. Convert on the way in.
