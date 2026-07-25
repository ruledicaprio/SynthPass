
# 📄 Classification-Reconstruction Learning for Open-Set Recognition

> **Published:** CVPR 2019  
> **arXiv:** [1812.04246](https://arxiv.org/abs/1812.04246)  
> **Authors:** Ryota Yoshihashi, Wen Shao, Rei Kawakami, Shaodi You, Makoto Iida, Takeshi Naemura  
> **Code:** [Official Repository](https://github.com/saketd403/CROSR) *(Community PyTorch Implementation)*

---

## 📑 Table of Contents
1. [Executive Summary (TL;DR)](#1-executive-summary-tldr)
2. [Core Concepts & Problem Statement](#2-core-concepts--problem-statement)
3. [The Key Insight: The Limitation of Pure Supervision](#3-the-key-insight-the-limitation-of-pure-supervision)
4. [Proposed Methodology: CROSR & DHRNet](#4-proposed-methodology-crosr--dhrnet)
5. [Loss Function & Training Strategy](#5-loss-function--training-strategy)
6. [Experimental Results & Visualizations](#6-experimental-results--visualizations)
7. [Conclusion & Real-World Impact](#7-conclusion--real-world-impact)
8. [References](#8-references)

---

## 1. Executive Summary (TL;DR)
Traditional deep learning classifiers operate under a "closed-world" assumption, meaning they assume all test samples belong to classes seen during training. In the real world, models encounter **unknown** (Out-of-Distribution, OoD) samples and often misclassify them with high confidence. 

This paper introduces **CROSR** (Classification-Reconstruction learning for Open-Set Recognition), a novel framework that jointly trains a network for *both* classification and reconstruction. By forcing the network to reconstruct the input from a compact latent representation, the model learns richer, more generalized features that prevent it from over-specializing to known classes. This allows CROSR to robustly detect unknown samples without sacrificing closed-set classification accuracy.

---

## 2. Core Concepts & Problem Statement

| Term | Definition |
| :--- | :--- |
| **Closed-Set Classification** | The model assumes all test data belongs to the $N$ known classes trained on. |
| **Open-Set Recognition (OSR)** | The model must correctly classify known classes *and* reject samples from unknown classes ($C_{N+1}$). |
| **Activation Vector (AV)** | The output of the final hidden layer ($\mathbf{y}$) before the Softmax layer. |
| **Openmax** | A baseline OSR method that calibrates the Softmax AV using Weibull distributions to estimate the probability of a sample belonging to an "unknown" class [[3]]. |

**The Problem:** Existing deep OSR methods rely entirely on features learned via *supervised classification*. These features are highly optimized to discriminate *between* known classes, but they discard detailed information about the input sample itself. Consequently, they are prone to assigning high confidence to unknown outliers that happen to activate the right neurons.

---

## 3. The Key Insight: The Limitation of Pure Supervision

The authors observed that Activation Vectors (AVs) are not the best representations for modeling class-belongingness $p(\mathbf{x} \in C_i)$. 
* Supervised networks are optimized to output correct class probabilities $p(C_i | \mathbf{x})$, but they are **not encouraged to encode information about $\mathbf{x}$ itself**.
* Therefore, testing whether $\mathbf{x}$ is a probable member of $C_i$ using only the AV is insufficient. 
* **Solution:** Exploit **reconstructive latent representations** ($\mathbf{z}$), which inherently encode more detailed information about the input $\mathbf{x}$, to complement the classification prediction $\mathbf{y}$.

---

## 4. Proposed Methodology: CROSR & DHRNet

The framework consists of two main innovations:

### 4.1. CROSR (Classification-Reconstruction for Open-Set Recognition)
Instead of using only the AV ($\mathbf{y}$) to measure distance to class centroids, CROSR concatenates the classification prediction $\mathbf{y}$ with the reconstructive latent representation $\mathbf{z}$:

$$ d(\mathbf{x}, C_i) = \left\| [\mathbf{y}, \mathbf{z}] - \boldsymbol{\mu}_i \right\|_2 $$

Where $[\mathbf{y}, \mathbf{z}]$ is the concatenated vector, and $\boldsymbol{\mu}_i$ is the mean of this concatenated vector for class $C_i$ in the training set. This joint distribution forms a tighter hypersphere per class, making it easier to spot outliers that fall outside.

### 4.2. DHRNet (Deep Hierarchical Reconstruction Net)
To effectively provide both $\mathbf{y}$ and $\mathbf{z}$, the authors designed DHRNet. 
* **The Flaw of Standard Autoencoders:** Compressing the entire network into a bottleneck destroys the expressive power needed for large-scale classification.
* **The Flaw of Ladder Networks:** They use lateral connections to pass details, but they do not compress these details into compact latent variables, making high-dimensional outlier detection difficult (due to the "curse of dimensionality" or concentration on the sphere).
* **The DHRNet Solution:** It extracts a series of compact latent representations ($\mathbf{z}_1, \mathbf{z}_2, ..., \mathbf{z}_L$) from *multiple intermediate stages* of the classification network via bottlenecked lateral connections. 

This is analogous to "overhauling" a mechanical product: disassembling the input $\mathbf{x}$ into decomposed factors ($\mathbf{z}_l$), checking each part for anomalies, and reassembling them. The final $\mathbf{z}$ is the concatenation of these multi-level bottlenecks, providing a rich, compact signature of the input.

---

## 5. Loss Function & Training Strategy

The network is trained **only on known classes** using a joint loss function:

$$ \mathcal{L}_{total} = \mathcal{L}_{classification} + \mathcal{L}_{reconstruction} $$

1. **$\mathcal{L}_{classification}$**: Standard Softmax cross-entropy between the prediction $\mathbf{y}$ and the ground-truth labels.
2. **$\mathcal{L}_{reconstruction}$**: $\ell^2$ distance (for images) or cross-entropy (for text) between the original input $\mathbf{x}$ and the reconstructed output $\tilde{\mathbf{x}}$ generated by the decoder from $\mathbf{z}$.

*Crucially, the reconstruction loss acts as a data-dependent regularizer, preventing the network's representations from over-specializing to the known classes.*

---

## 6. Experimental Results & Visualizations

The authors evaluated CROSR on five standard datasets (MNIST, CIFAR-10, SVHN, TinyImageNet, DBpedia) under two protocols: **Class Separation** and **Outlier Addition**.

### 6.1. Performance on CIFAR-10 (Matching `plot_cvpr3_rev.pdf`)
The uploaded plot shows the relationship between the rejection threshold and the F1-score for CIFAR-10 (with ImageNet-crop as outliers). 
* **DHRNet + CROSR (ours)** consistently dominates all baselines across all thresholds.
* **LadderNet + Openmax** outperforms standard Supervised + Openmax, proving that reconstruction regularization helps. However, **DHRNet + CROSR** is superior because it explicitly uses compact latent factors rather than just adding a reconstruction loss.

| Method | ImageNet-crop | ImageNet-resize | LSUN-crop | LSUN-resize |
| :--- | :---: | :---: | :---: | :---: |
| Supervised + Softmax | 0.639 | 0.653 | 0.642 | 0.647 |
| Supervised + Openmax | 0.660 | 0.684 | 0.657 | 0.668 |
| LadderNet + Openmax | 0.653 | 0.670 | 0.652 | 0.659 |
| **DHRNet + CROSR (Ours)** | **0.721** | **0.735** | **0.720** | **0.749** |

### 6.2. Visualizing Confidence Sorting (Matching `sorted.pdf`, `sorted_cifar_3.pdf`, `samples_right.pdf`, `samples_shao.pdf`)
The uploaded snippets correspond to **Figure 6** in the paper, which sorts test samples by the model's final confidence score (highest confidence on the left). 
* **Cyan boxes:** Misclassified known samples.
* **Red boxes:** Unknown (OoD) samples.
* **Observation:** Baseline models (Supervised + Openmax) are easily "deceived" by unknown samples (e.g., placing Omniglot characters or CIFAR "Deer"/"Ship" outliers high in the confidence ranking). 
* **CROSR's Advantage:** DHRNet + CROSR successfully pushes these unknown samples to the right (lower confidence), only being deceived by outliers that have a very high semantic similarity to the inlier data (e.g., MNIST-noise).

### 6.3. Robustness to Diverse Outliers (MNIST)
CROSR showed massive improvements on challenging outliers:
* **MNIST vs. Omniglot:** F1-score jumped from 0.680 (Supervised+Openmax) to **0.793** (CROSR).
* **MNIST vs. MNIST-noise:** F1-score jumped from 0.720 to **0.827**.

---

## 7. Conclusion & Real-World Impact

1. **First of its Kind:** This is the first work to demonstrate that deep *reconstruction-based* representation learning is highly effective for open-set recognition, challenging the paradigm that purely discriminative (supervised) features are best.
2. **Architectural Innovation:** DHRNet successfully bridges the gap between high-accuracy classification and compact, anomaly-sensitive latent representations via bottlenecked lateral connections.
3. **Practicality:** CROSR achieves state-of-the-art results *without* requiring complex, unstable Generative Adversarial Networks (GANs) to synthesize fake "unknown" training data. The computational overhead at test time is negligible (~3–5 ms/image).

**Takeaway:** If you want an AI to know what it *doesn't* know, you must force it to understand how to *rebuild* what it sees, not just slap a label on it.

---

## 8. References

1. **Yoshihashi et al., 2019.** Classification-Reconstruction Learning for Open-Set Recognition. *CVPR*. [[18]]
2. **Bendale & Boult, 2016.** Towards open set deep networks. *CVPR*. (Introduced Openmax). [[3]]
3. **Rasmus et al., 2015.** Semi-supervised learning with ladder networks. *NIPS*. [[31]]
4. **Hendrycks & Gimpel, 2017.** A baseline for detecting misclassified and out-of-distribution examples in neural networks. *ICLR*. [[14]]
5. **Neal et al., 2018.** Open set learning with counterfactual images. *ECCV*. (GAN-based baseline). [[27]]

--- 
*Document generated based on the CVPR 2019 paper "Classification-Reconstruction Learning for Open-Set Recognition" and the provided visual/data snippets.*