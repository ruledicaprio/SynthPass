
# 📄 The Devil is in the Wrongly-Classified Samples: Towards Unified Open-Set Recognition

> **Published:** ICLR 2023  
> **arXiv:** [2302.04002v1 [cs.CV]](https://arxiv.org/abs/2302.04002)  
> **Code:** [GitHub Repository](https://github.com/Jun-CEN/Unified_Open_Set_Recognition)  
> **Authors:** Jun Cen et al. *(Equal contribution)*

---

## 📑 Table of Contents
1. [Executive Summary (TL;DR)](#1-executive-summary-tldr)
2. [Core Concepts & Definitions](#2-core-concepts--definitions)
3. [Key Discovery: The "Devil" in InW Samples](#3-key-discovery-the-devil-in-inw-samples)
4. [The Feature vs. Uncertainty Paradox](#4-the-feature-vs-uncertainty-paradox)
5. [Training Settings Analysis](#5-training-settings-analysis)
6. [Proposed Solution: Few-Shot UOSR & FS-KNNS](#6-proposed-solution-few-shot-uosr--fs-knns)
7. [Experimental Benchmarks](#7-experimental-benchmarks)
8. [Conclusion & Real-World Impact](#8-conclusion--real-world-impact)
9. [References](#9-references)

---

## 1. Executive Summary (TL;DR)
In real-world AI deployments, a model should not only reject completely unknown objects (Out-of-Distribution, **OoD**) but also reject known objects it is about to **misclassify** (In-Distribution Wrong, **InW**). This stricter, more practical task is called **Unified Open-Set Recognition (UOSR)**. 

The authors made a groundbreaking discovery: **Standard AI models are accidentally much better at this strict UOSR task than the traditional, easier Open-Set Recognition (OSR) task.** This is because when an AI is uncertain, its internal "confidence score" for a *misclassified* known object looks almost identical to its score for a *completely unknown* object. The paper formalizes this, analyzes training strategies, and proposes **FS-KNNS**, a state-of-the-art method for Few-Shot UOSR.

---

## 2. Core Concepts & Definitions

To understand the paper, test data is divided into three distinct buckets:
| Acronym | Name | Definition | UOSR Goal |
| :--- | :--- | :--- | :--- |
| **InC** | In-Distribution Correct | Known objects the AI classifies *correctly*. | ✅ **Accept** |
| **InW** | In-Distribution Wrong | Known objects the AI classifies *incorrectly*. | ❌ **Reject** |
| **OoD** | Out-of-Distribution | Completely unknown objects/classes. | ❌ **Reject** |

### Task Comparison
The paper distinguishes UOSR from related uncertainty-estimation tasks based on their ground-truth uncertainty targets (`0` = Accept, `1` = Reject):

| Task | InC | InW | OoD | Requires InD Classification? |
| :--- | :---: | :---: | :---: | :---: |
| **Selective Prediction (SP)** | 0 | 1 | ✗ (Ignored) | ✅ Yes |
| **Anomaly/Outlier Detection** | 0 | 0 | 1 | ❌ No |
| **Open-Set Recognition (OSR)** | 0 | 0 | 1 | ✅ Yes |
| **Unified OSR (UOSR)** | 0 | **1** | **1** | ✅ Yes |
| **Model Calibration (MC)** | Probabilistic | Probabilistic | ✗ (Ignored) | ✅ Yes |

> 💡 **Key Difference:** OSR aims to *accept* InW samples (treating them as valid InD), while UOSR aims to *reject* them alongside OoD samples.

---

## 3. Key Discovery: The "Devil" in InW Samples

When the authors applied existing OSR methods to the UOSR benchmark, they found a massive paradox: **The methods scored significantly higher on the harder UOSR task than the easier OSR task.**

*   **The Phenomenon:** The uncertainty distribution of **InW** samples is extremely close to **OoD** samples, and very far from **InC** samples.
*   **The Consequence:** Traditional OSR methods are designed to flag high-uncertainty objects as "unknown." Therefore, they accidentally flag misclassified objects (InW) as unknown too. This *hurts* their OSR score (since OSR says InW should be accepted), but makes them **accidentally perfect for UOSR** (where InW *should* be rejected).
*   **Impact:** Without InW samples, OSR performance increases by a large margin (e.g., SoftMax AUROC jumps from 75.59% to 84.69%). Existing OSR works had dismissed this phenomenon, but this paper proves InW samples are the primary cause of false positives in OSR.

---

## 4. The Feature vs. Uncertainty Paradox

*Deep dive from Appendix D: Why do InW samples act like OoD samples in uncertainty, but not in features?*

The authors analyzed the feature representations and found a fascinating contradiction:
1. **Feature Space:** InW samples are **more similar** to InC samples than to OoD samples. (They form a hierarchy: InC is closest to training data, InW surrounds InC, and OoD is on the outer edge).
2. **Uncertainty Space:** Despite feature similarity, the mathematical functions used to estimate uncertainty (like Softmax) project InW scores to look like OoD scores. 

> 🧮 **Mathematical Formulation:**  
> Let $x_c, x_w, x_o$ be features of InC, InW, and OoD.  
> Feature similarity: $\text{sim}(x_w, x_c) > \text{sim}(x_w, x_o)$  
> Uncertainty similarity: $\text{sim}(f(x_w), f(x_c)) < \text{sim}(f(x_w), f(x_o))$  
> *Conclusion:* The uncertainty estimation function $f$ inherently changes the similarity relationship, grouping "I don't know" and "I am guessing wrong" into the same low-confidence bucket.

---

## 5. Training Settings Analysis

The paper evaluates two popular training techniques to see how they affect UOSR:

| Technique | Mechanism | Effect on InC/OoD | Effect on InC/InW | Overall UOSR Impact |
| :--- | :--- | :---: | :---: | :---: |
| **Pre-training** | Initialize with large-scale weights (e.g., ImageNet). | ✅ Improves | ✅ **Improves** | 🌟 **Highly Beneficial** (Improves both closed-set accuracy and uncertainty discrimination). |
| **Outlier Exposure (OE)** | Train with unlabeled "junk" OoD data as a proxy. | ✅ Improves | ❌ **Worsens/Neutral** | ⚠️ **Mixed** (Great for OSR, but can hurt UOSR because it fails to help the model recognize its own InW mistakes). |

*Note: Real outlier data is consistently more beneficial than generated/synthetic outlier data (e.g., VOS).*

---

## 6. Proposed Solution: Few-Shot UOSR & FS-KNNS

In real life, you might have 1 to 5 reference photos of a new, unknown threat. The authors introduce **Few-Shot UOSR**, where $1$ or $5$ samples per OoD class are provided during evaluation.

### The Problem with Baselines
A standard KNN baseline (**FS-KNN**) using these reference samples is great at finding OoD objects, but its **InC/InW performance is severely harmed** (e.g., dropping from 89.58% to 79.58% AUROC). This is the key difference between Few-Shot OSR and Few-Shot UOSR.

### The Solution: FS-KNNS (Few-Shot KNN with SoftMax)
Inspired by SIRC, the authors propose dynamically fusing the **SoftMax** uncertainty score ($u_0$) with the **FS-KNN** uncertainty score ($u_1$). 

The fused uncertainty $\hat{u}$ is calculated as:
$$ \hat{u} = u_0 + \frac{1}{1 + e^{-\alpha(u_1 - \lambda_{\text{knns}})}} u_1 $$

*   **$\lambda_{\text{knns}}$**: A threshold (set to $\mu - \beta \cdot \sigma$ of the OoD reference distribution) that determines when to apply the FS-KNN weight.
*   **$\alpha$**: A coefficient controlling the rate of change.
*   **Result:** This keeps the high InC/OoD performance of FS-KNN while retaining the comparable InC/InW performance of the SoftMax baseline, achieving **State-of-the-Art (SOTA)** across all settings.

---

## 7. Experimental Benchmarks

The authors built a comprehensive benchmark across multiple domains. Key takeaways from the results:

*   **Image Domain (CIFAR-100 / TinyImageNet / LSUN):** UOSR AUROC consistently outperforms OSR AUROC across all backbones (ResNet50, VGG13).
*   **Video Domain (UCF101 / HMDB51 / MiT-v2):** The phenomenon holds true for video backbones (TSM, I3D) as well.
*   **Semantic Shift Benchmark (SSB):** Even in fine-grained, challenging datasets (CUB, FGVC-Aircraft) where OoD samples share coarse labels with InD, the InW/OoD AUROC remains close to 50%, proving the uncertainty overlap is a fundamental property of neural networks.
*   **Noisy Outlier Exposure:** The model's open-set performance remains surprisingly robust even when up to 100% of the outlier exposure data is corrupted with InD labels, especially when using frameworks like NGC to correct them.

---

## 8. Conclusion & Real-World Impact

If you are deploying AI in the real world (self-driving cars, medical diagnosis, security), you care about **safety**, not academic OSR benchmarks. 

This paper proves that you don't necessarily need wildly new architectures to make AI safer. Standard AI confidence metrics are already naturally wired to reject both "things I've never seen" and "things I'm about to get wrong." By shifting the evaluation metric from OSR to **UOSR**, and utilizing methods like **FS-KNNS**, we get a much more accurate, practical picture of how safe an AI actually is in the wild.

---

## 9. References

*(Selected key references cited in the paper)*

1. **Deng et al., 2009.** ImageNet: A large-scale hierarchical image database. *CVPR*.
2. **Scheirer et al., 2013.** Toward open set recognition. *IEEE TPAMI*.
3. **Hendrycks & Gimpel, 2017.** A baseline for detecting misclassified and out-of-distribution examples in neural networks. *ICLR*.
4. **Kim et al., 2021.** A unified benchmark for the unknown detection capability of deep neural networks. *arXiv:2112.00337*.
5. **Hendrycks et al., 2019a.** Using pre-training can improve model robustness and uncertainty. *ICML*.
6. **Hendrycks et al., 2019b.** Deep anomaly detection with outlier exposure. *ICLR*.
7. **Vaze et al., 2022.** Open-set recognition: A good closed-set classifier is all you need. *ICLR*.
8. **Xia & Bouganis, 2022.** Augmenting softmax information for selective classification with out-of-distribution data (SIRC). *arXiv:2207.07506*.
9. **Sun et al., 2022.** Out-of-distribution detection with deep nearest neighbors. *ICML*.

--- 
*Document generated based on the ICLR 2023 paper "The Devil is in the Wrongly-Classified Samples: Towards Unified Open-Set Recognition".*