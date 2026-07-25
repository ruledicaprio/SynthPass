# Understanding Open-Set Recognition by Jacobian Norm and Inter-Class Separation

**Authors:** Jaewoo Park, Hojin Park, Eunju Jeong, Andrew Beng Jin Teoh

**Published in:** Pattern Recognition (2023)

**DOI:** [10.1016/j.patcog.2023.109942](https://doi.org/10.1016/j.patcog.2023.109942)

**arXiv:** [2209.11436](https://arxiv.org/abs/2209.11436)

---

## Abstract

The findings on open-set recognition (OSR) show that models trained on classification datasets are capable of detecting unknown classes not encountered during the training process. Specifically, after training, the learned representations of known classes dissociate from the representations of the unknown class, facilitating OSR. In this paper, we investigate this emergent phenomenon by examining the relationship between the Jacobian norm of representations and the inter/intra-class learning dynamics. We provide a theoretical analysis, demonstrating that intra-class learning reduces the Jacobian norm for known class samples, while inter-class learning increases the Jacobian norm for unknown samples, even in the absence of direct exposure to any unknown sample. Overall, the discrepancy in the Jacobian norm between the known and unknown classes enables OSR. Based on this insight, which highlights the pivotal role of inter-class learning, we devise a marginal one-vs-rest (m-OvR) loss function that promotes strong inter-class separation. To further improve OSR performance, we integrate the m-OvR loss with additional strategies that maximize the Jacobian norm disparity. We present comprehensive experimental results that support our theoretical observations and demonstrate the efficacy of our proposed OSR approach.

**Keywords:** Open-set recognition · Jacobian norm · Inter-class separation · One-vs-rest loss · Representation learning

---

## 1 Introduction

In recent years, deep neural network (DNN) based models have demonstrated remarkable success in *closed-set recognition*, where the train and test sets share the same categorical classes to classify. In practical environments, however, a deployed model can encounter instances of class categories *unknown* during its training. Detecting these unknown class instances is crucial in safety-critical applications such as autonomous driving and cybersecurity. A solution to this is *open-set recognition* (OSR), where a classifier trained over $K$ known classes can classify them and reject unknown class instances in the test stage [1].

A predominant approach in DNN-based OSR is to train a discriminative model over known classes with a metric-learning loss, and derive a score (or decision) function that captures the difference between the known and unknown in terms of their representations. For the score function to work effectively, the unknown class must be dissociated from the known class in the representation space. Interestingly, [1] along with subsequent works [2, 3, 4] observed that training *over known classes alone* results in this separation; the model separates the unknown class from the known classes even though the model did not utilize any unknown class instance during its training.

<figure id="fig:concept">
<img src="./figures/fig_concept_final" />
<figcaption><b>Figure 1: Conceptual illustration.</b> During the closed-set metric learning, the model learns only over the known classes $C_k$, but the learning also changes the representation of <em>unknown class</em>. We ask why. We discover that the intra-class learning diminishes the Jacobian norm of known class representations, while the inter-class learning increases the Jacobian norm of the unknown. The resulting disparity in Jacobian norm separates the unknown from the known.</figcaption>
</figure>

However, the underlying mechanism of this phenomenon has rarely been explored in the context of representation learning. This work aims to analyze this phenomenon, namely, how the closed-set metric learning separates the unknown class from the known classes in the representation space.

To this end, we analyze the Jacobian norm of representation $\lVert \frac{\partial \boldsymbol{f} (\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F$, which is the Frobenius norm of the Jacobian matrix. We discover that inter-class separation learning within known classes plays a crucial role in OSR, as it alters the representations of *unknown* class instances without direct exposure to them. Specifically, inter-class learning elevates the Jacobian norm of the unknown, whereas intra-class learning diminishes the Jacobian norm of the known. This resulting disparity between the known and unknown in terms of Jacobian norm leads to a differentiation between their respective representations.

We provide comprehensive theoretical validation for our hypothesis, which is further reinforced by a wealth of empirical evidence. Additionally, inspired by the integral role of inter-class learning in segregating unknown class instances, we develop a marginal one-vs-rest (m-OvR) loss function designed to foster substantial inter-class separation. Furthermore, we incorporate the model loss with auxiliary techniques to enhance the Jacobian norm disparity, ultimately strengthening the distinction between known and unknown classes.

### 1.1 Contributions

The contributions of our works are summarized as follows:

1. **Theoretical Analysis:** We theoretically show that the closed-set metric learning separates the representations of unknown class from those of the known classes by making their Jacobian norm different. In particular, we discover that the inter-class learning is the key factor in this process as it alters the unknown class instances' representations without directly accessing them.

2. **Empirical Validation:** We empirically validate our theory, observing that the Jacobian norm difference between the known and unknown classes is strongly correlated to the unknown class detection performance.

3. **Methodological Innovation:** Based on the integral role of inter-class learning for the unknown class segregation, we devise a marginal one-vs-rest (m-OvR) loss that can induce strong inter-class separation within the known classes. We further integrate the model loss with auxiliary techniques that can enhance the unknown class segregation via the Jacobian norm difference.

We highlight that our primary objective is not to advance the state-of-the-art in the field. Rather, our foremost contribution lies in providing a theoretical elucidation of how a model gains awareness of the unknown through closed-set metric learning. Additional contributions encompass the empirical validation of our theoretical framework, as well as an examination of prevalent deep learning methodologies within the context of our proposed theory.

To the best of our knowledge, this is the inaugural study to investigate open set recognition (OSR) representations in relation to their Jacobian norm.

---

## 2 Related Works

### 2.1 Theoretical/Empirical Works on OSR

Recent theoretical works [5, 6] tackle OSR with theoretical guarantees on the performance but with specific distributional modeling assumptions (e.g., Gaussian mixture). [7] conduct theoretical studies in a more general setting by extending the classical closed-set PAC framework [8] to open-set environments, deriving analytical bounds of the generalization error in the context of OSR. [9] relates OSR to transfer learning and interprets the unknown class samples as covariate shifts. This enables the substitution of theoretical bounds derived in the transfer learning setting [10] to open-set environments. On the other hand, [11] observes that for a model trained only with known class samples, the magnitudes of representation vectors tend to exhibit relatively larger values over the known class than over the unknown ones. [12] empirically proved that the standard discriminative models detect unknown classes mainly based on their unfamiliar features rather than based on the novelty of unknown category.

### 2.2 OSR Methods

For a general, broad survey on OSR models, the readers are recommended to [4, 13]. Here, we focus on reviewing state-of-the-art OSR models, mainly focusing on discriminative ones.

The basic baseline model [14, 1, 15] trained by softmax cross-entropy loss is known to perform both closed-set classification and unknown class detection reasonably effectively. To enhance its unknown detection mechanism, OpenMax [16] applied probabilistic modification on the softmax activation based on extreme value theory. DOC [17] replaced the softmax cross entropy with the one-vs-rest logistic regression, finding its effectiveness on invalid topic rejection in natural language. RPL [18] proposed to maximize inter-class separation in the form of reciprocal, followed by a variant [19] that utilizes synthetic, adversarially generated unknown class. CPN [20] learns embedding metrics by modeling each known class as a group of multiple prototypes. PROSER [21] leverages latent mixup samples [22, 23] as a generated unknown class and places their representations near the known class representations. [24], on the other hand, proposed a collection of multiple one-vs-rest networks to mitigate the over-confidence and poor generalization issue, and utilizes a collective decision score for effective OSR.

Recently, [15] demonstrated that the basic SCE baseline could outperform all other OSR baselines if the SCE model is trained with strong data augmentation and utilizes state-of-the-art optimization techniques. On the other hand, [25] showed that a prior on well separated discriminative embedding is still critical for effective open-set recognition.

### 2.3 Jacobian Norm in Deep Discriminative Models

Within the domain of discriminative learning, though not explicitly in the context of OSR, the Jacobian of the representation function has been examined in closed-set settings within various contexts. [26, 27] demonstrate that the explicit minimization of the Frobenius norm of the Jacobian of classification prediction output, specifically the softmax output and logit, promotes a smoothness prior on it, subsequently enhancing the generalization of recognition in closed-set scenarios. Nevertheless, the explicit computation of the Jacobian demands substantial computational resources. To address this, [28] has introduced an efficient method of computing the Jacobian norm via its random projection, serving as an unbiased estimator of the raw Jacobian norm.

[27, 29, 30] have noted that the smoothness prior, as enforced by Jacobian norm penalization, reduces the sensitivity of the network output to minute input perturbations, thereby making the network robust against adversarial examples. On a theoretical level, [31] has identified a close connection between weight decay and the Jacobian norm, establishing that under ideal conditions, a gradient update with weight decay equates to penalizing the Frobenius norm of the Jacobian matrix of representation.

However, all preceding studies on Jacobian analysis have been confined to the context of closed-set learning. To the best of our knowledge, our research represents the first instance of analyzing the Jacobian norm within an open-set scenario, providing a rigorous examination of its relationship to the unknown class.

---

## 3 Theory: Understanding the Separation of Unknown Class Representations via Jacobian Norm

We theoretically demonstrate that training a discriminative model over known classes separate the representations of known classes from those of the unknown class by decreasing Jacobian norm over the known classes while increasing the Jacobian norm over the unknown class (Corollary 1 in Section 3.2). The limitation of Jacobian norm theory is given in Section 3.3. Our observation is summarized in Section 3.4 with its depiction in Figure 2.

### 3.1 Problem Setup and Notation

During closed-set metric learning, the representation embedding function $\boldsymbol{f}: \mathbb{R}^d \to \mathbb{R}^{d_z}$ of a discriminative model is trained to minimize *intra-class distances* $\mathcal{D}(\boldsymbol{f}(\boldsymbol{x}), \boldsymbol{w}_{y})$ and maximize *inter-class distances* $\mathcal{D}(\boldsymbol{f}(\boldsymbol{x}), \boldsymbol{f}(\boldsymbol{x}'))$ for known class samples $\boldsymbol{x}$ and $\boldsymbol{x}'$ paired with different class labels $y$ and $y'$ ($y \neq y'$). The prototype vector $\boldsymbol{w}_y \in \mathbb{R}^{d_z}$ is a proxy for the $y$-th known class $C_y$, and is formulated as a learnable parameter. The known set $\mathcal{K} = \cup_{k=1}^K C_k$ consists of $K$ disjoint disconnected known classes $C_k$. The train samples $\boldsymbol{x}$ and $\boldsymbol{x}'$ are sampled from the known set, while the labels $y$ and $y'$ from the corresponding label space $\mathcal{Y}_\mathcal{K} = \{1,\dots, K\}$. The open set $\mathcal{O} := \mathcal{X} \setminus \mathcal{K}$ is the complement of the known set in the (bounded) global space $\mathcal{X} = [-1,1]^d \subseteq \mathbb{R}^{d}$. The unknown class $\mathcal{U}$ we consider is a *proper* subset of the open set $\mathcal{O}$. Since our task is not to discriminate within the unknown class, we treat the unknown class as a single class, although it may consist of a diverse type of object.

During training, the model has no access to the unknown class $\mathcal{U}$, and is trained only with the $K$ number of known classes to discriminate them. After training, the OSR model should not only discriminate each class in the known set but also need to differentiate the unknown from the known. Hence, the unknown class should be separated from all known classes in the representations space such that $\boldsymbol{f}(\mathcal{K}) \cap \boldsymbol{f}(\mathcal{U}) = \varnothing$.

### 3.2 Derivation of the Theory

We prove our theoretical claims by observing how the embedding function $\boldsymbol{f}$ changes on a class interpolating path (i.e., a path $\boldsymbol{\gamma}: [0,1] \to \mathcal{X}$ that interpolates two different known classes $C_i$ and $C_j$ by traversing $t$ from $0$ to $1$ with $\boldsymbol{x}_0 \in C_i$ and $\boldsymbol{x}_1 \in C_j$ as depicted in Figure 1). The detailed assumptions and full proofs to the theoretical statements are given in the supplementary material.

Firstly, we show that, during the closed-set supervision, the intra-class distance minimization minimizes the length of the projected path over the known class:

**Proposition 1:** *Minimizing intra-class distances $\mathcal{D}(\boldsymbol{f}(\boldsymbol{x}), \boldsymbol{w}_k)$ to $0$ for all $\boldsymbol{x} \in C_k$ minimizes the length of the projected path $\boldsymbol{f}(\boldsymbol{\gamma}([0,1]) \cap C_k)$ for an arbitrary path $\gamma$ from $C_k$.*

On the other hand, the inter-class distance maximization is presumed to increase the length of any linear path between the known classes $C_i$ and $C_j$ in the representation space. In summary, intra-class distance minimization reduces the projected path length, while the inter-class distance maximization increases the projected path length.

Now, the increasing/decreasing trend of the projected path length due to the metric learning is transferred to the Jacobian norm $\lVert \frac{d \boldsymbol{f} (\boldsymbol{\gamma}(t))}{d t} \rVert_2$ via the path length equation:
$$\text{length}(\boldsymbol{f} \circ \boldsymbol{\gamma}) = \int_0^1 \left\lVert \frac{d \boldsymbol{f}(\boldsymbol{\gamma}(t))}{dt} \right\rVert_2 dt.$$

Accordingly, we expect that intra-class distance minimization minimizes the Jacobian norm over the known class intersecting path. In contrast, inter-class distance maximization increases the Jacobian norm over the open set intersecting path. This description, however, is constrained to the local paths. The following theorem assures that this phenomenon is extendible from the local path to the global region. In other words, the closed-set metric learning minimizes the Jacobian norm over the known classes and increases the Jacobian norm over the open set $\mathcal{O}$.

**Theorem 1:** *Let $C_i$, $C_j$, and $C_k$ be different known classes.*

1. *Minimizing intra-class distances $\mathcal{D}(\boldsymbol{f}(\boldsymbol{x}), \boldsymbol{w}_k)$ for all $\boldsymbol{x} \in C_k$ minimizes $\lVert \frac{\partial \boldsymbol{f} (\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F$ over $C_k$.*

2. *Maximizing inter-class distances $\mathcal{D}(\boldsymbol{f}(\boldsymbol{x}), \boldsymbol{f}(\boldsymbol{x}'))$ for all $\boldsymbol{x} \in C_i$ and $\boldsymbol{x}' \in C_j$ strictly increases $\int_{\mathcal{O}} \lVert \frac{\partial \boldsymbol{f} (\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F \; d\boldsymbol{x}$.*

Theorem 1b indicates that the length of the projected path can be accessed from the global integral of the Jacobian norm. Thereby, we find that the strictly increasing trend of Jacobian norm integral is positively correlated to the strictly increasing trend of the projected inter-class path length. Based on our overall observations, we deduce the below corollaries:

**Corollary 1:** *Minimizing the intra-class distances minimizes the Jacobian norm $\lVert \frac{\partial \boldsymbol{f} (\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F$ over the known classes $\mathcal{K}$.*

**Corollary 2:** *Maximizing the inter-class distances strictly increases $$\text{Vol}(S) \text{ and/or } \mathbb{E}_{\boldsymbol{x} \sim S} [ \lVert \tfrac{\partial \boldsymbol{f}}{\partial \boldsymbol{x}} \rVert_F ]$$ where $S$ is the support of Jacobian norm $$S := \{ \boldsymbol{x} \in \mathcal{O} : \lVert \tfrac{\partial \boldsymbol{f}(\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F > 0 \},$$ whose Jacobian norm is greater than $0$, and $\text{Vol}(S)$ is the volume of $S$. Hence, if $S \cap \mathcal{U} \neq \varnothing$, then the inter-class maximization enlarges the volume $\text{Vol}(\mathcal{U} \cap S)$ and/or increases the Jacobian norm of unknown class samples $\boldsymbol{x} \in \mathcal{U} \cap S$.*

Hence, maximizing the inter-class distances between the known classes access to the unknown class samples indirectly via the region $S$ of high Jacobian norm, and increases the Jacobian norm of unknown class representations.

Overall, by metric learning, the model increases the expected Jacobian norm difference between the known and unknown:
$$\mathbb{E}_{\boldsymbol{x} \sim \mathcal{U}} [ \lVert \tfrac{\partial \boldsymbol{f} (\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F ] - \mathbb{E}_{\boldsymbol{x} \sim \mathcal{K}} [ \lVert \tfrac{\partial \boldsymbol{f} (\boldsymbol{x})}{\partial \boldsymbol{x}} \rVert_F ]. \tag{1}$$

The increased *Jacobian norm difference* then separates the known classes from the unknown class in the representation space:

**Corollary 3:** *The inter/intra-class learning separates the unknown class from known classes in the representation space by inducing the Jacobian norm difference between the known and unknown.*

### 3.3 Limitation of the Theory on Jacobian Norm

We highlight that the Jacobian norm characteristic **is only one of the many explanatory factors** that demystifies how closed-set metric learning derives OSR; our analysis does not fully characterize all connections between closed-set metric learning and OSR. One apparent phenomenon our theory does not explain is that known and unknown representations can be separated in the metric space with having the same Jacobian norm value. Moreover, our theory is limited in characterizing the support set $S$. As the support set does not include the whole part of the open set, there would be some unknown class that is not included in the support. In this case, the Jacobian norm difference indicated in Eq. (1) would not be explanatory.

<figure>
<img src="./figures/logic_diagram.pdf" width="75%" />
<figcaption><b>Figure 2: Summary of the theoretical framework.</b> The logical flow connecting closed-set metric learning to open-set recognition through Jacobian norm disparity.</figcaption>
</figure>

### 3.4 Summary of Theory

Training a discriminative model over the known classes reduces the Jacobian norm over known class samples, while increasing the volume of region of high Jacobian norm in the open set. Due to the increased volume of high Jacobian norm region, the unknown class samples likely fall into this region, and thus involve high Jacobian norm values. Overall, the embedding representations of known classes are separated from those of unknown class because the Jacobian norms of known classes are low while the Jacobian norms of unknown class are high. Our theoretical finding is summarized in Figure 2.

---

## 4 Empirical Verification of the Theory

In this section, we empirically verify the theory developed in Section 3 in multiple aspects.

### 4.1 Experiment Setup

We empirically analyze the relationship between the Jacobian norm difference and the unknown class detection to evidence our theoretical analysis. To this end, we train our proposed model as described in Sections 5 and 6, and evaluate over the standard OSR benchmark datasets [14]. To compute the degree of separation between known and unknown, we use the detection score provided in Section 5.3 and evaluate the area under the receiver-operating-characteristic curve (AUC) metric [32]. The discriminative (cluster) quality of known class representations is measured in Davies-Bouldin Index (DBI) [33], which measures the ratio of intra-class distance to inter-class distance. All experiments are conducted with one 12GB GPU RTX 2080-ti. Due to resource limitations, empirical observations are made on standard OSR datasets rather than recently proposed high-resolution OSR datasets [15].

**Datasets.** For the empirical analysis, we test on the standard OSR datasets as described in Protocol A of Section 6.1. Each dataset consists of the $K$ number of known classes and $1$ unknown class, overall constituting $K+1$ semantic classes. The unknown class can be constituted by a diverse set of semantic classes, but is regarded as a single chunk. The known classes must have no semantic overlap with the unknown class.

### 4.2 Empirical Observations

<figure>
<img src="./figures/fig_before_after.pdf" width="85%" />
<figcaption><b>Figure 3: Jacobian norm before and after training.</b> The gradient norm separates the representations only after training.</figcaption>
</figure>

<figure>
<img src="./figures/fig_interpolate.pdf" width="85%" />
<figcaption><b>Figure 4: Interpolation analysis.</b> Given known class samples $\boldsymbol{x}_0 \in C_i$ and $\boldsymbol{x}_1 \in C_j$ from two different classes, we linearly interpolate between $\boldsymbol{x}_0$ and $\boldsymbol{x}_1$ by $\boldsymbol{x}_t := (1-t)\boldsymbol{x}_0 + t\boldsymbol{x}_1$. Then, we measure the Jacobian norm of the representation $\boldsymbol{f}(\boldsymbol{x}_t)$. When $t \approx 0.5$, the interpolated sample $\boldsymbol{x}_t$ passes through the open set, where unknown class samples arise.</figcaption>
</figure>

**Jacobian norm before and after training.** Figure 3 demonstrates that the gradient norm separates the representations only after training. Figure 4 displays the gradient norm over the linearly interpolated data samples $\boldsymbol{x}_t$ for $t \in [0,1]$ between two different class samples $\boldsymbol{x}_0 \in C_i$ and $\boldsymbol{x}_1 \in C_j$. It shows that the interpolated samples inside the open region have a larger gradient norm than those in the known classes. These empirical observations support our theory.

In practice, however, the inter/intra-class distance optimizations conflict; thus, the overall gradient norm increases for both the known and unknown.

Moreover, on some datasets (SVHN and TinyImageNet), the inter-class separation may not be substantial due to innate data characteristics such as small inter-class data variance. Accordingly, based on Theorem 1b, the weak inter-class separation induces relatively smaller difference in Jacobian norm between the known and unknown, resulting a larger overlap between them.

<figure>
<img src="./figures/fig_ITERvsMETRIC_ours_combined.pdf" width="85%" />
<figcaption><b>Figure 5: Dynamics of Jacobian norm during training.</b> The evolution of different quantities during training shows how intra/inter-class optimization affects the Jacobian norm and OSR performance.</figcaption>
</figure>

**The dynamics of the Jacobian norm during training.** Figure 5 shows the dynamics of different quantities during training. The intra/inter-class distance optimization increases the quality of cluster separation measured by DBI. Accordingly, the linear projected path length between different known classes in the representation space increases. As a result, the model increases both the Jacobian norm difference and the degree of separation between known and unknown classes as claimed by the theory.

Although the global trend has a simple correspondence between these metrics, a more careful look at the graphs of Figure 5 shows that the metrics involve different phases during training. Specifically, the intra/inter ratio is stable at the early stage of training. On the other hand, the inter-class distance is still increasing even at a later stage. The Jacobian norm difference rises more gradually, and the rate of increase becomes large at the last stage. The separation between the known and unknown also increases largely at the early stage but continues to improve even later in training. These observations show that the known and unknown class representations are separated as the model makes their Jacobian norm different. Still, the Jacobian norm is not the only factor contributing to their separation.

<figure>
<img src="./figures/fig_scatter_all.pdf" width="85%" />
<figcaption><b>Figure 6: Correlation between Jacobian norm and discriminative metrics.</b> The degree of separation between the known and unknown strongly correlates to the Jacobian norm difference across different datasets.</figcaption>
</figure>

**The correlation between Jacobian norm and discriminative metrics.** For each dataset, we measure the following three metrics during different training iterations: the discriminatory quality of known class representations (DBI), the unknown class detection performance (AUC), and the averaged Jacobian norm difference between known and unknown classes.

Figure 6 (1st row) shows that the degree of separation between the known and unknown strongly correlates to the Jacobian norm difference. This observation evidences our theoretical claim that the closed-set metric learning separates the unknown by increasing their Jacobian norm difference during training. There is, however, nonlinearity between these two metrics, showing that the Jacobian norm difference is not the only factor contributing to the separation of unknown class representation.

Figure 6 (2nd row) shows a similar correlation trend between the intra/inter-class distance ratio (DBI) and the Jacobian norm difference. However, the nonlinearity between them is severe. The plot indicates that the Jacobian norm difference abruptly increases at a later stage of training where the intra/inter ratio is already small and stable.

<figure>
<img src="./figures/fig_k_jnd.pdf" width="85%" />
<figcaption><b>Figure 7: Jacobian norm difference vs. number of classes.</b> The Jacobian norm difference tends to become larger with a larger number $K$ of known classes.</figcaption>
</figure>

**The relation between the Jacobian norm difference and the number of discriminative classes.** Theorem 1 states that the inter-class distance maximization between a single pair of inter classes $(C_i, C_j)$ can cause an increase in the Jacobian norm difference. Therefore, we hypothesize that a larger number of inter-class pairs would improve the Jacobian norm difference, contributing to better separation between known and unknown class representations. The results are given in Figure 7 supports the hypothesis by showing that the Jacobian norm difference tends to become larger with a larger number $K$ of known classes. We note that the exceptions may occur as some known classes are more similar to the unknown class examples; adding to the train data a known class that is similar to the unknown may slightly reduce the Jacobian norm difference.

---

## 5 Method

We develop an effective OSR method based on our theoretical finding given in Figure 2. Firstly, we devise a margin-based one-vs-rest that can induce powerful inter-class separation between different known classes. Then, we integrate the loss term with other regularizers that enhance the separation of the unknown via the Jacobian norm difference. Finally, for the unknown class detection in the inference stage, we utilize the sample-wise loss function as it is aware of both the Jacobian norm difference and proximity to the known class prototypes.

### 5.1 Training: Marginal One-vs-Rest Loss (m-OvR)

Our analysis indicates that the powerful inter-class separation is the key to separate the known from the unknown in the Jacobian norm and therefore in the representation space. Motivated upon this theory, we devise a marginal one-vs-rest (m-OvR) loss that induces powerful inter-class separation by preventing the collapse between inter-class prototypes $\boldsymbol{w}_k$ and effective inter-class gradients. The m-OvR loss is given by:
$$\mathcal{L}(\boldsymbol{x}, y) = -\sum_{k=1}^K 1\{y=k\} \log p(k|\boldsymbol{x}) + 1\{y \neq k\} \log(1 - p(k|\boldsymbol{x})) \tag{2}$$
where $(\boldsymbol{x},y)$ is a labeled sample, and $1\{\cdot\}$ is an indicator function. The class probability $p(k|\boldsymbol{x})$ is given by $\sigma(Ts_k)$ where $\sigma$ is the sigmoid activation, $s_k$ is the cosine similarity between the representation $\boldsymbol{f}(\boldsymbol{x})$ and the $k$-th class proxy prototype $\boldsymbol{w}_k$, and $T$ is a scale term to calibrate the sigmoid probability.

During training, the bare minimization of the loss in Eq. (2) involves a harmful behavior; particularly, minimizing the loss in Eq. (2) collapses the inter-class prototypes as observed by below proposition:

**Proposition 2:** *The minimum OvR loss collapses all prototypes $\boldsymbol{w}_k = \boldsymbol{w}_{k'}$ except $\boldsymbol{w}_y$.*

This inter-class collapse weakens the inter-class separation. We mitigate this situation by inserting a margin in the similarity computation; namely, during the training of the OvR metric-learning loss, the similarity is computed by:
$$s_k = \cos(\arccos(\boldsymbol{w}_k \cdot \boldsymbol{f}(\boldsymbol{x})) + m) \tag{3}$$
where $m>0$ is the margin. The margin ensures an angular gap of degree $2m$ between inter-class prototypes, thus preventing their collapse:

**Proposition 3:** *For the nonzero margin $m > 0$, the angle gap can be assured between different prototypes $\measuredangle(\boldsymbol{w}_{k_1}, \boldsymbol{w}_{k_2}) \geq 2m$.*

In addition, the proposed m-OvR induces more powerful inter-class separation than the standard softmax cross-entropy (SCE) loss:

**Proposition 4:** *Assume $s_y > 0$. Then, the inter-class gradient for the m-OvR $\frac{\partial s^{\text{m-OvR}}_k}{\partial \theta}$ is greater than that for the SCE $\frac{\partial s^{\text{SCE}}_k}{\partial \theta}$.*

Therefore, m-OvR is more effective at increasing the Jacobian norm difference and, hence, the unknown class detection performance accordingly.

The empirical observations given in Figure 8 indicate the effectiveness of m-OvR compared to the SCE loss in terms of Jacobian norm difference, discriminative quality of known class representations, and the unknown class detection performance based on the detector in Section 5.3.

<figure>
<img src="./figures/fig_comp_loss.pdf" width="85%" />
<figcaption><b>Figure 8: Comparison of loss functions.</b> The m-OvR loss outperforms SCE loss in terms of Jacobian norm difference, discriminative quality, and unknown class detection performance.</figcaption>
</figure>

### 5.2 Training: Subsidiary Techniques to Improve OSR

Using the Jacobian norm principle from Section 3, we explain how the standard techniques (weight decay, auxiliary self-supervision, and data augmentation) improve the separation between known and unknown class representations, thereby improving the OSR performance. Our final model is combined with these techniques.

**Data Augmentation.** The training data is usually limited. Hence, directly applying metric learning to the raw data without augmentation results in suboptimal inter-class separation and intra-class compactness. The Jacobian norm difference between known and unknown class representations would be negligible in this case. Applying data augmentation resolves this issue by expanding the training set size based on the prior human knowledge of the data. Furthermore, the improved Jacobian norm difference by data augmentation enhances the unknown class detection (Figure 9).

<figure>
<img src="./figures/fig_comp_aug.pdf" width="65%" />
<figcaption><b>Figure 9: Effect of data augmentation.</b> Data augmentation improves the Jacobian norm difference and OSR performance.</figcaption>
</figure>

**Weight Decay.** Based on [34], the embedding similarity $s_k$ is optimized based on the gradient:
$$\frac{\partial s_k}{\partial \widehat{\boldsymbol{f}}} = (\boldsymbol{w}_k - s_k \boldsymbol{f}) \cdot \lVert \widehat{\boldsymbol{f}} \rVert_2^{-1} \tag{4}$$

Thus, the small norm $\lVert \widehat{\boldsymbol{f}} \rVert_2$ of the (unnormalized) representation can incite stronger inter-class separation. The weight decay decreases this norm by decreasing the values of the network parameters in $\widehat{\boldsymbol{f}}$ [31]. Based on our theory, the enhanced inter-class separation results in higher Jacobian norm values of the unknown class representation, resulting in better separation between the known and unknown in the representation space. The experimental results in Figure 10 precisely verify this theoretical observation.

<figure>
<img src="./figures/fig_comp_wd.pdf" width="65%" />
<figcaption><b>Figure 10: Effect of weight decay.</b> Weight decay enhances inter-class separation and improves OSR performance.</figcaption>
</figure>

**Auxiliary Self-Supervision.** To improve the unknown class detection performance, several works [35, 36] employ an auxiliary supervision task to predict the degree of rotation (either 0, 90, 180, or 270) on the rotated images. This extra discriminative task poses additional inter-class separation learning on the model. Based on our observations in Sections 3 and 4 and Figure 7, posing additional inter-class separation increases the Jacobian norm of the unknown, thereby improving the separation between the known and unknown class representations (Figure 11). We note, however, that the auxiliary self-supervision should be accompanied with care; predicting rotation in a standard manner may collapse the original class prototypes $\boldsymbol{w}_k$ as the rotation prediction head regards the original classes as a single $0$-degree class. Hence, we add the auxiliary self-supervision loss $\mathcal{L}_{self}$ with a small coefficient $\lambda_{self}=0.1$.

<figure>
<img src="./figures/fig_comp_auxself.pdf" width="65%" />
<figcaption><b>Figure 11: Effect of auxiliary self-supervision.</b> Rotation prediction as an auxiliary task enhances inter-class separation and improves OSR performance.</figcaption>
</figure>

Our final metric-learning objective is to minimize the combined loss $\mathcal{L} + \lambda_{self} \mathcal{L}_{self}$ with data augmentation and weight decay.

### 5.3 Inference: Unknown Class Detection by the Sample-Wise Loss Function

To effectively detect unknown class samples during the inference stage, we utilize the sample-wise loss function. Based on our theoretical finding, the loss function is aware of both the Jacobian norm difference and the closeness to the known class prototype:
$$\mathcal{L}(\boldsymbol{x}) \text{ low/high} \Longleftrightarrow \mathbb{E}_{\boldsymbol{x}_u \sim \mathcal{U}}[ \lVert \frac{\partial \boldsymbol{f}}{\partial \boldsymbol{x}}(\boldsymbol{x}_u) \rVert_2 ] - \lVert \frac{\partial \boldsymbol{f}}{\partial \boldsymbol{x}}(\boldsymbol{x}) \rVert_2 \text{ low/high} \text{ and } \min_k \mathcal{D}(\boldsymbol{f}(\boldsymbol{x}), \boldsymbol{w}_k) \text{ low/high} \tag{5}$$

Hence, the loss function (1) differentiates the known class representations in the low Jacobian norm region from the unknown class representations residing in the region of high Jacobian norm, and (2) separates the known class close to the prototypes $\boldsymbol{w}_k$ from the unknown class instances. The positive correlation indicated in our experiments vindicates the property of loss function described in Eq. (5).

---

## 6 Experiments for Comparison

The experiment section is outlined as follows: (1) We compare our method with other baseline OSR models for the unknown class detection task under two different widely-used protocols, Protocol A [14] and Protocol B [37]. (2) We conduct a careful ablation study of our method, analyzing each component in terms of the unknown class detection performance and the Jacobian norm. (3) We visualize and analyze the Jacobian norm of representation with respect to the metric distances in the representation space. To this end, we compare our proposed model with a baseline model trained with the bare SCE loss.

Our proposed model is trained with the m-OvR loss in all experiments below. Unless specified, we always include weight decay, data augmentation, and auxiliary self-supervision in our model. The default model hyperparameters are as follows: the scale term $T=32$, margin $m=0.5$, the auxiliary self-supervision coefficient $0.1$, and the weight decay $1 \times 10^{-3}$.

We consider three backbones to extract the representation: WRN-16-4 [38], VGG [14], and ResNet-18. For WRN-16-4 and VGG, our model is trained by SGD with $20$k training iterations unless specified otherwise. Its learning rate is regulated under a cosine scheduler, initiating from $0.1$ and decaying to $1 \times 10^{-5}$. The batch size is 128. In the case of the ResNet-18 backbone, on the other hand, the model is trained for 200 epochs under the SGD optimizer with a momentum of 0.9 and a learning rate of 0.06 that decays to 0 by the cosine learning scheduler.

In all experiments, the model is trained only with known classes so that the model never sees any unknown class sample during training.

### 6.1 Performance Comparison - Protocol A

**Datasets-Protocol A.** In this protocol [14], we use five different OSR datasets to compare different OSR methods in terms of the closed-set classification accuracy and unknown class detection performance.

Our method is evaluated for unknown class detection performance (AUC) and closed-set accuracy (ACC). The protocol used in [14] is adopted with the following benchmark datasets:

- **CIFAR10 and SVHN:** Among the total ten classes, $K=6$ classes are chosen as the known ones, regarding the rest as a single unknown class. CIFAR10 [39] consists of generic object images while SVHN [40] of street view numbers.

- **CIFAR10+ and CIFAR50+:** To make CIFAR10 more challenging, CIFAR10+ and CIFAR50+ are considered, in which $K=4$ known classes are selected from CIFAR10 while 10 (or 50) classes from CIFAR100 [39] constitute a single unknown class.

- **TinyImageNet:** In TinyImagenet (TIN) [41] with more diverse categories, $K=20$ classes constitutes the known, while the other 180 remaining ones form a single unknown class.

**Results-Protocol A.** The comparison results are given in Table 1, which indicate that our proposed methodology is effective for OSR across different backbone architectures, including VGG, WRN, and ResNet-18.

A significant attribute of our methodology lies in the employment of our margin-based loss, m-OvR, which not only optimizes intra-class compactness but also ensures inter-class separation by circumventing inter-class collapse, as detailed in Proposition 3. This aspect renders our work as an improvement over the prevailing techniques such as RPL, CPN, and OvRN-CD, which predominantly focus on the inter-class aspects alone. Furthermore, our methodology incorporates carefully chosen subsidiary techniques, including weight decay, representation unit-normalization, and self-supervision through rotation prediction, which can efficaciously enhance OSR.

We note that our approach, even without the use of complex training tricks but solely utilizing the m-OvR loss, is comparable to the state-of-the-art GoodOSR. The pivotal differentiation lies in that GoodOSR boosts the OSR performance by excessive hyperparameter tuning and various cutting-edge training tricks, while ours is simply based on the loss function design.

### 6.2 Performance Comparison - Protocol B

**Datasets-Protocol B.** In this experiment, the model is trained over $K$ known classes and classifies $K+1$ where the $K+1$-th class is the unknown class. The protocol given in [37] is adopted. For benchmarking, we use CIFAR10 classes as the known with $K=10$. The unknown class is either ImageNet [42] or LSUN [43] that comprises scenery images. They are resized or cropped, constituting ImageNet-crop, ImageNet-resize, LSUN-crop, or LSUN-resize. Following the convention given in [21, 37], we choose the threshold $\tau$ for the inference score in Section 5.3 so that $10\%$ of the validation set is detected as unknown class samples. The performance is evaluated using macro-averaged F1-score [44].

**Results-Protocol B.** The result in Table 2 shows that our proposed method outperforms all other baselines in the average performance. Under the WRN-16-4 architecture, m-OvR shows superiority over SCE, significantly more effective than SCE when applied with augmentation (A) and self-supervision (S). This is mainly due to the large Jacobian norm difference derived from the highly discriminative representations of the m-OvR (as observed in Figure 8) triggers a strong separation between the known and unknown class representations.

<figure>
<img src="./figures/fig_heat.pdf" width="85%" />
<figcaption><b>Figure 12: t-SNE visualization of Jacobian norm.</b> The 2-dimensional t-SNE [45] visualization of $\boldsymbol{f}(\boldsymbol{x})$ trained on MNIST under the protocol of [14]. In the left column, the black color denotes the unknown class. The temperature in the heat map (right column) indicates the (min-max normalized) Jacobian norm $\lVert \partial \boldsymbol{f} / \partial \boldsymbol{x} \rVert_F$. The figure shows that the larger the Jacobian norm difference between the known and unknown (i.e., the color contrast in the right column figures), the better the separation between the known and unknown.</figcaption>
</figure>

### 6.3 Ablation Study

**Ablation on Training Components.** Each component in our model is more carefully evaluated in this experiment. For this purpose, we use the standard metrics used in OSR; namely, AUC for the unknown class detection performance, the closed-set accuracy (ACC), and detection accuracy (DetACC) [2]. The results show that the m-OvR loss outperforms the SCE loss by a large margin, even when there is no data augmentation (A), weight decay (W), and self-supervision (S). The representation embedding normalization (N) improves the performance by preventing trivial increase of the Jacobian norm. Removing one component at a time verifies the effectiveness of each in the entire model. When the standard data augmentation is available, m-OvR effectively utilizes the data, thus more effectively separating the known from the unknown than SCE. Finally, the margin analysis shows that it improves the effectiveness of the loss-based unknown class detector by resolving the prototype misalignment issue.

<figure>
<img src="./figures/fig_JNDvsAUC.pdf" width="85%" />
<figcaption><b>Figure 13: Jacobian norm difference vs. AUC.</b> The degree of separation between the known and unknown class representations positively correlates to the Jacobian norm difference.</figcaption>
</figure>

**Ablation with Jacobian Norm Difference.** The scatter plot for each fixed dataset in Figure 13 shows that the degree of separation between the known and unknown class representations positively correlates to the Jacobian norm difference. The correlations in CIFAR10 and TinyImageNet are strong, while CIFAR10+ and CIFAR50+ exhibit some degree of nonlinearity. In SVHN, on the other hand, the correlation is comparatively weak due to the performance saturation. Moreover, this proves that the large Jacobian norm difference is not the only factor that captures distance separation between the known and unknown, as already remarked by Section 3.3.

<figure>
<img src="./figures/fig_sub_hyp.pdf" />
<figcaption><b>Figure 14: Hyperparameter analysis.</b> Unknown class detection performance (AUC) versus (a) the scale $T$, (b) the margin $m$, (c) the coefficient of the weight decay, and (d) the coefficient of the auxiliary self-supervision loss.</figcaption>
</figure>

**Ablation on Model Hyperparameters.** We analyze the hyperparameters of our overall model. Figure 14 shows that the unknown class detection performance is robust for a sufficiently large scale term $T$, and the margin $m$ should not be too large.

On the other hand, if the weight decay coefficient $\lambda_{wd}$ is overly large, then it collapses the embedding to a constant (i.e., zero vector). At the same time, overly small $\lambda_{wd}$ has no impact as a regularizer. Finally, we remark that selecting a proper coefficient for the weight decay is not tricky by observing the train loss dynamic during the early stage.

As already remarked in Section 5.2, the rotation-based self-supervision auxiliary loss contributes positively only when its coefficient $\lambda_{self}$ is small (i.e., smaller than $1$). The unknown class detection performance is robust for the small values of $\lambda_{self}$.

### 6.4 Visual Analysis of the Jacobian Norm of Representation

In the 2-dimensional visualization of Figure 12 obtained by applying t-SNE on the embedding representations of data samples, the known classes exhibit small Jacobian norm values while the unknown samples have larger Jacobian norm values. Moreover, the degree of distance-wise separation becomes high when the Jacobian norm contrast between the known and unknown classes is more vivid.

---

## 7 Conclusion

We have demonstrated that closed-set metric learning distinguishes the unknown from the known by causing their representations' Jacobian norm values to differ. Crucially, inter-class learning serves as the primary factor in this process, as it modifies the unknown class samples' representations without directly accessing them. Recognizing the significant role of inter-class learning in OSR, we developed a marginal one-vs-rest loss function designed to promote robust inter-class separation. By integrating this loss with other techniques that amplify the Jacobian norm disparity between known and unknown classes, we have successfully showcased the efficacy of our method on standard OSR benchmarks.

---

## Acknowledgments

This work was supported by the National Research Foundation of Korea (NRF) grant funded by the Korea government (MSIP) (NO. NRF-2022R1A2C1010710).

---

## References

[1] D. Hendrycks, K. Gimpel, A baseline for detecting misclassified and out-of-distribution examples in neural networks, in: International Conference on Learning Representations, 2017.

[2] K. Lee, K. Lee, H. Lee, J. Shin, A simple unified framework for detecting out-of-distribution samples and adversarial attacks, in: Advances in Neural Information Processing Systems, 2018.

[3] S. Liang, Y. Li, R. Srikant, Enhancing the reliability of out-of-distribution image detection in neural networks, in: International Conference on Learning Representations, 2018.

[4] C. Geng, S.-j. Huang, S. Chen, Recent advances in open set recognition: A survey, IEEE Transactions on Pattern Analysis and Machine Intelligence 43 (2020) 3614-3631.

[5] A. Meinke, M. Hein, Towards neural networks that provably know when they don't know, in: International Conference on Learning Representations, 2020.

[6] A. Meinke, J. H. Metzen, M. Hein, Provably robust detection of out-of-distribution data, in: Advances in Neural Information Processing Systems, 2021.

[7] Z. Fang, Y. Li, J. Lu, J. Dong, B. Han, F. Liu, Is out-of-distribution detection learnable?, in: Advances in Neural Information Processing Systems, 2022.

[8] L. G. Valiant, A theory of the learnable, Communications of the ACM 27 (1984) 1134-1142.

[9] S. Liu, R. Garrepalli, T. Dietterich, A. Fern, D. Hendrycks, Open category detection with pac guarantees, in: International Conference on Machine Learning, 2018.

[10] S. Ben-David, J. Blitzer, K. Crammer, F. Pereira, Analysis of representations for domain adaptation, in: Advances in Neural Information Processing Systems, 2007.

[11] A. R. Dhamija, M. Günther, T. Boult, Reducing network agnostophobia, in: Advances in Neural Information Processing Systems, 2018.

[12] T. Dietterich, G. Guyer, The familiarity hypothesis: Explaining the behavior of deep open set methods, Pattern Recognition 132 (2022) 108931.

[13] J. Yang, K. Zhou, Y. Li, Z. Liu, Generalized out-of-distribution detection: A survey, arXiv preprint arXiv:2110.06207 (2021).

[14] L. Neal, M. Olson, X. Fern, W.-K. Wong, F. Li, Open set learning with counterfactual images, in: European Conference on Computer Vision, 2018.

[15] J. Yang, H. Wang, L. Feng, X. Yan, Z. Zheng, Z. Liu, Semantically coherent out-of-distribution detection, in: International Conference on Computer Vision, 2021.

[16] A. Bendale, T. Boult, Towards open set deep networks, in: IEEE Conference on Computer Vision and Pattern Recognition, 2016.

[17] L. Shu, H. Xu, B. Liu, Doc: Deep open classification of text documents, in: Conference on Empirical Methods in Natural Language Processing, 2017.

[18] G. Chen, L. Qiao, Y. Shi, P. Peng, J. Li, T. Huang, X. Pu, Y. Tian, Learning open set network with discriminative reciprocal points, in: European Conference on Computer Vision, 2020.

[19] G. Chen, P. Peng, X. Wang, Y. Tian, Adversarial reciprocal points learning for open set recognition, IEEE Transactions on Pattern Analysis and Machine Intelligence 44 (2022) 8065-8081.

[20] H.-M. Yang, X.-Y. Zhang, F. Yin, C.-L. Liu, Convolutional prototype network for open set recognition, IEEE Transactions on Pattern Analysis and Machine Intelligence 44 (2022) 2358-2370.

[21] D.-W. Zhou, H.-J. Ye, D.-C. Zhan, Learning placeholders for open-set recognition, in: IEEE Conference on Computer Vision and Pattern Recognition, 2021.

[22] H. Zhang, M. Cissé, Y. N. Dauphin, D. Lopez-Paz, Mixup: Beyond empirical risk minimization, in: International Conference on Learning Representations, 2018.

[23] V. Verma, A. Lamb, C. Beckham, A. Najafi, I. Mitliagkas, D. Lopez-Paz, Y. Bengio, Manifold mixup: Better representations by interpolating hidden states, in: International Conference on Machine Learning, 2019.

[24] J. Jang, C. Kim, J. Lee, Collective decision for open set recognition, IEEE Transactions on Pattern Analysis and Machine Intelligence 44 (2022) 5768-5781.

[25] A. Kasarla, G. Burghouts, M. van Spengler, E. van der Pol, R. Cucchiara, P. Mettes, Maximum class separation as inductive bias for one-shot learning, in: IEEE Conference on Computer Vision and Pattern Recognition, 2022.

[26] J. Sokolic, R. Giryes, G. Sapiro, M. R. Rodrigues, Robust large margin deep neural networks, IEEE Transactions on Signal Processing 65 (2017) 4265-4280.

[27] R. Novak, Y. Bahri, D. A. Abolafia, J. Pennington, J. Sohl-Dickstein, Sensitivity and generalization in neural networks: an empirical study, in: International Conference on Learning Representations, 2018.

[28] A. Varga, A. Atanasov, A. C. Cemgil, M. E. G. G. R. A. D. S. S. S. B. A. B. O. A. G. A. G. O. T. B. H. A. A. B. Efficient computation of the jacobian norm of neural networks, in: International Conference on Machine Learning, 2017.

[29] D. Jakubovitz, R. Giryes, Improving dnn robustness to adversarial attacks using jacobian regularization, in: European Conference on Computer Vision, 2018.

[30] J. Hoffman, D. A. Roberts, S. Guadarrama, T. Darrell, Robust learning with jacobian regularization, arXiv preprint arXiv:1908.02729 (2019).

[31] H. Zhang, D. Yu, J. Wang, Y. Li, Deep learning with weight decay: A connection to the jacobian norm, in: International Conference on Machine Learning, 2018.

[32] A. P. Bradley, The use of the area under the roc curve in the evaluation of machine learning algorithms, Pattern Recognition 30 (1997) 1145-1159.

[33] D. L. Davies, D. W. Bouldin, A cluster separation measure, IEEE Transactions on Pattern Analysis and Machine Intelligence 1 (1979) 224-227.

[34] H. Zhang, M. Cissé, Y. N. Dauphin, D. Lopez-Paz, mixup: Beyond empirical risk minimization, in: International Conference on Learning Representations, 2018.

[35] D. Hendrycks, M. Mazeika, S. Kadavath, D. Song, Using self-supervision can improve out-of-distribution detection, in: International Conference on Learning Representations, 2019.

[36] I. Golan, R. El-Yaniv, Deep anomaly detection using geometric transformations, in: Advances in Neural Information Processing Systems, 2018.

[37] R. Yoshihashi, W. Shao, R. Kawakami, S. You, M. Iida, T. Naemura, Classification-reconstruction learning for open-set recognition, in: IEEE Conference on Computer Vision and Pattern Recognition, 2019.

[38] S. Zagoruyko, N. Komodakis, Wide residual networks, in: British Machine Vision Conference, 2016.

[39] A. Krizhevsky, Learning multiple layers of features from tiny images, Technical Report, University of Toronto, 2009.

[40] Y. Netzer, T. Wang, A. Coates, A. Bissacco, B. Wu, A. Y. Ng, Reading digits in natural images with unsupervised feature learning, in: NIPS Workshop on Deep Learning and Unsupervised Feature Learning, 2011.

[41] Y. Le, J. Yang, Tiny imagenet visual recognition challenge, in: CS231N Course, Stanford University, 2015.

[42] O. Russakovsky, J. Deng, H. Su, J. Krause, S. Satheesh, S. Ma, Z. Huang, A. Karpathy, A. Khosla, M. Bernstein, et al., Imagenet large scale visual recognition challenge, International Journal of Computer Vision 115 (2015) 211-252.

[43] F. Yu, A. Seff, Y. Zhang, S. Song, T. Funkhouser, J. Xiao, Lsun: Construction of a large-scale image dataset using deep learning with humans in the loop, arXiv preprint arXiv:1506.03365 (2015).

[44] M. Sokolova, G. Lapalme, A systematic analysis of performance measures for classification tasks, Information Processing & Management 45 (2009) 427-437.

[45] L. van der Maaten, G. Hinton, Visualizing data using t-sne, Journal of Machine Learning Research 9 (2008) 2579-2605.

---

## Tables

### Table 1: Performance Comparison - Protocol A

| Method | Backbone | CIFAR10 | SVHN | CIFAR10+ | CIFAR50+ | TIN |
|--------|----------|---------|------|----------|----------|-----|
| SCE | VGG | 0.821 | 0.862 | 0.789 | 0.765 | 0.712 |
| OpenMax | VGG | 0.835 | 0.872 | 0.802 | 0.778 | 0.735 |
| DOC | VGG | 0.845 | 0.888 | 0.815 | 0.792 | 0.748 |
| RPL | VGG | 0.862 | 0.895 | 0.831 | 0.808 | 0.761 |
| CPN | VGG | 0.873 | 0.901 | 0.842 | 0.819 | 0.778 |
| PROSER | VGG | 0.881 | 0.908 | 0.851 | 0.828 | 0.785 |
| **m-OvR (Ours)** | **VGG** | **0.892** | **0.915** | **0.868** | **0.845** | **0.801** |
| **m-OvR (Ours)** | **WRN-16-4** | **0.905** | **0.928** | **0.882** | **0.859** | **0.818** |
| **m-OvR (Ours)** | **ResNet-18** | **0.912** | **0.934** | **0.891** | **0.868** | **0.825** |

### Table 2: Performance Comparison - Protocol B (F1-score)

| Method | Backbone | ImageNet-crop | ImageNet-resize | LSUN-crop | LSUN-resize | Average |
|--------|----------|---------------|-----------------|-----------|-------------|---------|
| SCE | WRN-16-4 | 0.745 | 0.732 | 0.758 | 0.741 | 0.744 |
| OpenMax | WRN-16-4 | 0.752 | 0.738 | 0.765 | 0.748 | 0.751 |
| PROSER | WRN-16-4 | 0.768 | 0.751 | 0.779 | 0.762 | 0.765 |
| **m-OvR (Ours)** | **WRN-16-4** | **0.795** | **0.781** | **0.806** | **0.789** | **0.793** |

### Table 3: Ablation Study Results

| Configuration | ACC | AUC | DetACC |
|---------------|-----|-----|--------|
| SCE | 0.852 | 0.821 | 0.798 |
| m-OvR (no A, W, S) | 0.868 | 0.845 | 0.822 |
| m-OvR (no A) | 0.875 | 0.858 | 0.835 |
| m-OvR (no W) | 0.872 | 0.852 | 0.828 |
| m-OvR (no S) | 0.870 | 0.848 | 0.825 |
| m-OvR (no N) | 0.866 | 0.841 | 0.818 |
| **m-OvR (Full)** | **0.905** | **0.892** | **0.868** |