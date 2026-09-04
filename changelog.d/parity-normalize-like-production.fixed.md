The Tier-2 parity harness now normalises extractions the way the pipeline does. It applied no
normalisation to the model's output, while both pipeline entry points normalise every Tier-2
result, so it scored values the product never emits and reported failures that do not exist. The
corrected measurement is 48.1% of prompt fields on the standard corpus and 39.5% with the MRZ
held out, against 26.5% and 21.0% previously recorded.
