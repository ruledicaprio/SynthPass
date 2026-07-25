
***

# Class-Specific Semantic Reconstruction for Open Set Recognition

CONVENTIONAL deep neural networks (DNNs) are trained based on a closed-set assumption, in which the test classes are all seen during training.
In real-world applications, the test samples may come from unknown classes [@yoshihashi2019classification]. When meeting such an unknown sample,
traditional DNNs will compulsorily classify it as one of the known classes and make a wrong prediction, which may lead to irreparable losses in
certain critical scenarios, such as medical diagnosis and autonomous driving.

Open set recognition (OSR) addresses this challenge by making the models correctly classify the samples from known classes (i.e., the closed set)
and accurately identify those from unknown classes (i.e., the open set) [@geng2020recent]. The main challenge for OSR is that no information about
unknown classes is available during training, making it difficult to distinguish known and unknown classes (i.e., reduce open space risk) [@scheirer2013toward].
Traditional DNNs emphasize the discriminative features of known classes, and learn a partition of the entire feature space. This leads to a serious problem:
samples of unknown classes are still located in certain specific regions and, thus, are identified as known classes with high confidence [@prototype].
Consequently, many previous works have proposed to learn compact representations of known classes, so that the model can separate closed and open set spaces. Among these methods, auto-encoder (AE)-based methods [@oza2019c2ae; @oza2019deep; @sun2020conditional; @yoshihashi2019classification; @sun2020open] and prototype-like methods [@chen2020learning; @prototype; @chen2021adversarial] are currently the most powerful.

As illustrated in (c), AE-based methods learn latent representations by reconstructing the raw input to retain most information from an image.
As an AE can learn to reconstruct the images of known classes during training, test images originating from unknown classes would lead to high
reconstruction errors, and thus, can be recognized [@oza2019c2ae; @sun2020conditional]. This can be considered learning a low-dimensional manifold
to fit the distribution of known samples. To classify known classes, these methods learn a classifier based on latent representations obtained through
pixel-wise reconstruction of the raw image. However, two problems remain in these methods: (1) classification degradation and (2) open space risk introduction.

In classification degradation, using the latent representations learned by AE for the classifier harms the performance of closed-set classification.
This occurs primarily because some unnecessary classification information (e.g., background information) is retained and interferes with the learning
of the classifier on recognizing known classes [@tian2020what]. Open space risk introduction refers to the fact that continuous manifold learned to fit
known samples might devour the inter-class regions and, thus, introduce open space risk (c).

Unlike AE-based methods, prototype-like methods, including generalized convolutional prototype learning [@prototype] and the recently proposed reciprocal
point learning [@chen2020learning], learn class-specific points to fit the extracted representations corresponding to the labeled class or rest classes,
respectively. The sense of these methods is straightforward. However, the prototype-learning framework still faces great challenges on the OSR task.
The main challenge is the class under-representation problem, where using only a single point or very few points cannot sufficiently represent the class. On the one hand, prototype learning assumes the Gaussian distribution for class-specific features [@prototype]. However, this is seldom satisfied in real-world applications, which would introduce the open space risk. On the other hand, intra-class features are compressed to a quite limited number of points in prototype-learning frameworks. This may cause the model to filter out certain necessary information that could help discriminate unknown classes [@chu2020distance].

To address the aforementioned problems of both AE-based and prototype-like learning methods, we fully exploit them in a proposed novel method,
called class-specific semantic reconstruction (CSSR). Specifically, CSSR reduces open space risk by modeling each known class with a specific AE manifold.
In CSSR, the feature of each sample is extracted using a DNN. Then, an individual AE is specified to each known class to project different classes to various
manifolds. The AEs are plugged into the top of the DNN backbone to reconstruct the semantic representations, rather than the raw image. As the AE manifold is
a learnable representation of specific categories, the reconstruction errors, signifying the point-to-manifold distance, are used as the logits for classification.
CSSR minimizes the cross-entropy loss, where the reconstruction error of the AE specified to the labeled category is minimized. A graphical description is shown in b.

Our proposed framework can help solve the aforementioned problems of existing methods. Compared to AE-based methods, the proposed CSSR approach (1) solves the problem
of classification degradation by discarding unnecessary information and reconstructing the semantic features rather than the raw image and (2) handles the open space 
risk problem by learning the class-specific manifolds to release the devoured inter-class regions. Compared to prototype-like methods, the proposed CSSR approach deals
well with the problem of class under-representation by learning a class-specific manifold. This not only breaks the Gaussian assumption of classes but also retains more
key information of classes than representing the class with a single point.

Through the end-to-end learning process, the class-specific AEs and DNN boost each other to identify the open space while learning highly class-related semantic
representations. The AEs tend to associate each class with a subset of semantic features. The sample of a known class tends to activate its related features while
inactivating unrelated ones. For samples of an unknown class, their semantic features are not activated as they are not related to any features of known classes.
This property is also exploited for detecting unknown classes. The results of the experiments conducted on various datasets show that the proposed method significantly
outperforms other methods, and improves the performance of both closed and open set recognition.

In summary, this study makes the following contributions:
1. We propose a simple yet effective method, namely CSSR, for open set recognition. It specifies an individual AE to each known class, and plugs such AEs into the top
of the DNN backbone to reconstruct the semantic representations learned by the backbone network. CSSR improves the fitting and representation learning ability,
thereby enhancing the open set performance.
2. We conduct a theoretical analysis to explain the open space risk produced by existing methods, and discuss the connections between CSSR and existing methods
to understand CSSR comprehensively.
3. We performed experiments under various protocols. The results demonstrate that CSSR can significantly outperform baseline methods and achieve state-of-the-art
performance on multiple public datasets. Typically, CSSR improves F1 score by an average of 8.3% on the task of open set recognition.

## Related Work

Our work is mainly related to open set recognition, particularly AE-based and prototype-like methods. Open set recognition is naturally related to some other problems
(i.e., out-of-distribution (OOD) detection [@hendrycks2016a] and novelty detection [@perera2019ocgan]). The OOD detection methods are briefly discussed in this section.

### Open Set Recognition
Early works utilized traditional machine learning methods. They used the scores produced by the classifiers, and the unknown samples could be identified by measuring
the similarity between the samples and known classes [@bendale2015towards; @junior2017nearest]. For example, Scheirer [@scheirer2013toward] employed a support vector
machine for known class identification and adopted the extreme value distribution to detect the unknowns. Recently, the powerful representation learning capability of DNNs
has been applied to detect the unknowns.

Several scholars have designed or utilized the classification layer for OSR [@hendrycks2016a; @bendale2016towards; @zhou2021learning]. A plain choice is to utilize the
maximum SoftMax probabilities and reject unconfident predictions [@hendrycks2016a]. Bendale [@bendale2016towards] proved SoftMax probability is not robust and proposed
to replace the SoftMax function with the OpenMax function, which redistributes the scores of SoftMax to obtain the confident score of the unknown class explicitly.
Zhou [@zhou2021learning] proposed the concept of placeholder learning, where the overconfident predictions are calibrated by reserving classifier placeholders for unknown
classes.

**AE-based DNN Methods.** Zhang [@zhang2017sparse] asserted that reconstruction errors contain useful discriminative information, and proposed to use sparse representation
to model the open set recognition problem. Yoshihashi [@yoshihashi2019classification] designed the CROSR method, which uses latent representations for closed set classifier training and unknown detection. Oza and Patel [@oza2019c2ae] proposed a two-step C2AE method. The method first trains the encoder for closed set identification, then keeps it fixed and adds the class-conditional information to train the decoder for unknown detection. Sun [@sun2020conditional] used a variational AE to force different latent features to approximate different Gaussian models for unknown detection. They subsequently developed CPGM [@sun2020open], which adds discriminative information into probabilistic generative models. Perera [@perera2020generative] fed the raw and reconstructed images to a classification network, and the prediction was simultaneously confident when the reconstruction was consistent with the raw input. However, AE-based methods suffer from two problems, as stated in Section 1. (1) The representation learned from pixelwise image reconstruction contains unnecessary background information. This might harm both close and open set performance. (2) AEs learn a continuous manifold to fit known samples, which might devour the inter-class regions.

**Prototype-like DNN Methods.** Yang [@prototype] proposed generalized convolutional prototype learning, which replaces the close-world assumed SoftMax classifier with
an open-world oriented prototype model. Chen [@chen2020learning] developed reciprocal point learning (RPL), which classifies a sample as known or unknown based on the otherness with reciprocal points. Subsequently, RPL was further improved to ARPL [@chen2021adversarial], integrating an extra adversarial training strategy to enhance the model distinguishability into the known and unknown classes by generating confusing training samples. Prototype-like methods are limited owing to the lack of fitting ability and representation diversity. We apply class-specific AEs to address this issue.

### OOD Detection
As first introduced by Hendrycks and Gimpel [@hendrycks2016a], OOD detection involves the detection of samples that do not belong to the training set.
Several methods have considered the problem where OOD samples are available during training[@lee2018a; @liang2018enhancing; @dhamija2018reducing; @liu2020energy;
@quintanilha2018detecting; @zisselman2020deep]. However, this is not congruent with our task, where only in-distribution data are accessible during training.
In the following, we mainly focus on the models trained without extra OOD data.

**Supervised Methods.** With a similar problem setting to open set recognition, these methods build OOD detectors upon a classification task.
Some methods seek better score functions, including maximum SoftMax probability [@hendrycks2016a], maximum logit scores [@hendrycks2019a],
and energy score [@liu2020energy]. Vyas [@vyas2018out] used an ensemble of leave-one-out classifiers to simulate OOD accessible training individually.
Sastry and Oore [@sastry2020detecting] proposed to characterize activity patterns with Gram matrices and score OOD-ness by calculating 
he element-wise deviation comparing the Gram matrices from the training data.

**Self-Supervised Methods.** These methods are relatively novel and exploit the well-learned representations by self-supervision.
Golan and El-Yaniv [@golan2018deep], and Hendrycks [@hendrycks2019using] considered the task of predicting image transformations
(i.e., rotating image to $0\degree, 90\degree, 180\degree, \text{and } 270\degree$), which has also been leveraged as an auxiliary task in [@tack2020csi].
Self-supervised contrastive learning has shown considerable success in unsupervised representation learning [@chen2020a; @grill2020bootstrap],
and is being applied to OOD detection [@tack2020csi; @winkens2020contrastive; @sehwag2021ssd]. They observed that representations obtained by contrastive
learning have distinct patterns between in- and out-distribution data. Although designed for unlabeled settings, these methods have also been extended for
supervised learning.

## Preliminaries
**Open Set Recognition.** Given a set of $n$ labeled instances, $\mathcal{X} = \{(\mathbf{x}_i,y_i)\}_{i=1}^n$, where $y_i \in \{1,...,m\}$ are the corresponding
labels of known classes. For open set recognition, the goal is to learn a model from $\mathcal{X}$ that classifies test samples into $m+1$ classes, i.e.,
one of the $m$ known classes or an unknown class indexed by $m+1$.

**Auto-Encoder.** An AE learns effective representations of a set of data in an unsupervised manner. With the bottleneck structure of cascaded encoder $f$ and decoder $g$,
AE is forced to compress high-dimensional input features to a low-dimension embedding space $H$ to adequately reconstruct the raw input, i.e., minimizing the reconstruction
error $|\mathbf{x} - g(f(\mathbf{x}))|_2^2$ for each input sample $\mathbf{x}$. The decoder $g$ learns a manifold $V=\{g(\mathbf{h})|\mathbf{h} \in H\}$,
while the encoder $f$ learns a mapping from the original feature space to the manifold $V$. AE-based open set recognition methods fit manifold $V$ to the
distribution of known class samples, where the reconstruction error is the distance metric between the input sample and manifold $V$.
Existing methods reconstruct the entire image and minimize pixel-wise reconstruction error. However, fitting background pixels (category irrelevant information)
is unhelpful for both close set and open set recognition; this has also been demonstrated by Zhang [@zhang2020hybrid], who reported that building a flow density
estimator on latent representation works better than on the raw image. Therefore, we build AEs on the latent space extracted by a backbone network.

**Prototype and Reciprocal Learning.** By defining class-specific points set $U_i$ for each category $i$, prototype learning [@prototype] assigns a test sample to the nearest prototype point, and samples that are far away from all prototype points are regarded as being from an unknown class. Formally, it models closed and open set recognition as follows:
$$
\begin{aligned}
p(y=i|\mathbf{x}, \mathcal{B}, U) \propto & \left(-\min_{\mathbf{u} \in U_i}{|\mathcal{B}(\mathbf{x}) - \mathbf{u}|^2_2} \right), \\
p(unknown|\mathbf{x}, \mathcal{B}, U) \propto & \min_i \min_{\mathbf{u} \in U_i}{|\mathcal{B}(\mathbf{x}) - \mathbf{u}|^2_2}, \label{eq:prototype}
\end{aligned}
$$
where $\mathcal{B}$ is the backbone network extracting the embedding feature from input $\mathbf{x}$. Contrarily, reciprocal learning [@chen2020learning] utilizes class-specific reciprocal point set $U_i$ to learn otherness, instead of belongingness, and considers samples close to all reciprocal points to be unknown, which is expressed as:
$$
\begin{aligned}
p(y=i|\mathbf{x}, \mathcal{B}, U) \propto & \sum_{\mathbf{u} \in U_i}{|\mathcal{B}(\mathbf{x}) - \mathbf{u}|^2_2}, \\
p(known|\mathbf{x}, \mathcal{B}, U) \propto & \max_i \max_{\mathbf{u} \in U_i}{|\mathcal{B}(\mathbf{x}) - \mathbf{u}|^2_2}. \label{eq:reciprocal}
\end{aligned}
$$
In the training phase, the model optimizes the cross-entropy loss on SoftMax normalized class probabilities. However, optimizing discriminative loss alone is ineffective. Therefore, both methods propose different regularization terms to manage open space risk and achieve better training. The prototype framework proposes a generative loss $\mathcal{L}_{pl}$ (also called prototype loss), which is the maximum likelihood regularization under the Gaussian mixture density assumption:
$$
\begin{aligned}
\mathcal{L}_{pl}(\mathbf{x},y;U,\mathcal{B})= \min_{\mathbf{u} \in U_y} |\mathcal{B}(\mathbf{x}) - \mathbf{u}|^2_2. \label{eq:prototypeloss}
\end{aligned}
$$
For the reciprocal learning framework, open space risk is bounded by the constraining variance of feature-to-reciprocal point distances. This is formalized by:
$$
\begin{aligned}
\mathcal{L}_{rp}(\mathbf{x},y;U,\mathcal{B})=\sum_{u\in U_y} |d(\mathcal{B}(\mathbf{x}),u)-R_y|_2^2, \label{eq:rp_reg}
\end{aligned}
$$
where $d(\cdot,\cdot)$ is a distance function and $R_y$ is a class-specific learnable margin. Both $\mathcal{L}_{pl}$ and $\mathcal{L}_{rp}$ introduce extra compactness constraints.
In this study, we observe that optimizing single discriminative cross entropy loss leads to inconsistent distribution between feature distribution and prototype points.
Refer to Section "Fitting Known" for a detailed discussion.

## Experiments

### Implementation Details

As CSSR modifies only the classification layer, various backbone networks can alternatively be used in implementing CSSR. Following Chen [@chen2020learning], we chose to train small-scale datasets
with a Wide-ResNet [@zagoruyko2016wide] whose depth, width, and dropout rate we set to 40, 4, and 0, respectively, i.e., WRN40-4. However, for larger-scale datasets (i.e., TinyImageNet),
we substituted the backbone with ResNet18 [@he2016deep] for efficiency. In the training phase, the stochastic gradient descent optimizer was used with momentum = 0.9.
The model was trained for 200 epochs with batch size fixed to 128. The learning rate was set to 0.4 initially, and then, dropped by a factor of 10 at 130 and 190 epoch. We set $|\gamma|=0.1$ for all experiments.
The AEs were implemented with linear encoders and linear decoders. To make AEs' embedding space $H$ bounded, we used $\tanh$ as the activation function. The dimension of the embedding space for AEs was set to 64 for the
ResNet18 and WRN40-4 architecture. The score integration weights were set to be $1$s equally. Previous methods used data augmentation techniques to improve open set discrimination.
Following the settings of previous work [@perera2020generative; @zhou2021learning], we apply a simple data augmentation technique in [@cubuk2020randaugment].
In addition to prototype CSSR, RCSSR was also implemented and evaluated for a comprehensive comparison.

### Comparison with State-of-the-art Results

#### Unknown Detection

The evaluation protocol defined in [@neal2018open] was employed. Five image datasets were used in this experiment: SVHN [@netzer2011reading], TinyImageNet [@pouransari2014tiny],
CIFAR10 [@krizhevsky09learning], CIFAR+10, and CIFAR+50. For SVHN and CIFAR10, six classes were randomly sampled as the known classes, and the remaining four classes were used as the unknown classes.
For TinyImageNet, 20 classes were sampled as the known classes, and the remaining 180 classes as the unknown classes. For the CIFAR+$M$ datasets, the model was trained on four non-animal classes
from CIFAR10 as known classes, whereas $M$ animal classes from the CIFAR100 dataset [@krizhevsky09learning] were randomly selected as unknown classes. A threshold-independent metric,
the area under the receiver operating characteristic (AUROC) curve, was used as the evaluation metric. It was calculated by plotting the true positive rate against the false positive rate by varying thresholds.
The AUROC value is "1" if the knowns and unknowns are completely separable. Following [@neal2018open], we averaged the results over five randomized trials.

We compared the frameworks related to our method, i.e., AE-based methods [@yoshihashi2019classification; @oza2019c2ae; @oza2019deep; @sun2020conditional; @perera2020generative]
and prototype-like methods [@prototype; @chen2021adversarial], as well as two recent methods [@neal2018open; @zhou2021learning], using different architectures.
The results are reported in Table 1; the values other than CSSR are obtained from [@chen2021adversarial; @prototype; @oza2019deep; @zhou2021learning].
Except for being slightly behind ARPL on CIFAR+10, CSSR outperforms all other approaches in the five datasets, especially on SVHN ($+1.2\%$), CIFAR+50 ($+1.0\%$), and TinyImageNet ($+4.1\%$).

**Table 1: AUROC comparison between different methods on unknown detection tasks. The best performance values are highlighted in bold.**

| Methods                               | SVHN     | CIFAR10  | CIFAR+10 | CIFAR+50 | TinyImageNet |
|-:-------------------------------------|-:-:------|-:-:------|-:-:------|-:-:------|-:-:----------|
| CROSR [@yoshihashi2019classification] | 89.9     | 88.3     | 91.2     | 90.5     | 58.9         |
| C2AE [@oza2019c2ae]                   | 92.2     | 89.5     | 95.5     | 93.7     | 74.8         |
| MLOSR [@oza2019deep]                  | 95.5     | 84.5     | 89.5     | 87.7     | 71.8         |
| CGDL [@sun2020conditional]            | 93.5     | 90.3     | 95.9     | 95.0     | 76.2         |
| GFROSR [@perera2020generative]        | 93.5     | 83.1     | 91.5     | 91.3     | 64.7         |
| GCPL [@prototype]                     | 92.6     | 82.8     | -        | -        | -            |
| ARPL [@chen2021adversarial]           | 96.7     | 91.0     | 97.1     | 95.1     | 78.2         |
| Plain Softmax                         | 88.6     | 67.7     | 81.6     | 80.5     | 57.7         |
| OSRCI [@neal2018open]                 | 91.0     | 69.9     | 83.8     | 82.7     | 58.6         |
| PROSER [@zhou2021learning]            | 94.3     | 89.1     | 96.0     | 95.3     | 69.3         |
| **CSSR**                              | **97.9** | **91.3** | **96.3** | **96.2** | **82.3**     |
| **RCSSR**                             | **97.8** | **91.5** | **96.0** | **96.3** | **81.9**     |

#### Open Set Recognition
In addition to detecting unknown classes, open set recognition requires a joint classification of known classes, while rejecting the unknowns. We followed the experimental
set-up devised by Yoshihashi [@yoshihashi2019classification], where the models were trained on the entire CIFAR10 as known classes. In the test phase, the samples from other
datasets were used as unknowns, i.e., ImageNet [@russakovsky2015imagenet] and LSUN [@yu2015lsun]. The two datasets were further cropped or resized to ensure that they had
the same image size as the known samples; 10,000 samples (to maintain the consistency with the CIFAR10 test set) were selected forming ImageNet-Crop (IMGN-C), ImageNet-Resize (IMGN-R),
LSUN-Crop (LSUN-C), and LSUN-Resize (LSUN-R). For a fair comparison, we used the version of the four datasets released by Liang [@liang2018enhancing]. The performance was evaluated by
macro-averaged F1-scores in 11 classes (including 10 known classes and the 1 unknown). The results are presented in Table 2. The values other than CSSR are taken
from [@zhou2021learning; @oza2019deep; @sun2020conditional]. It can be observed that CSSR models outperformed existing methods by a large margin ($8.3\%$ on average).

**Table 2: Open set classification results on the CIFAR-10 dataset with various unknown datasets added in the test phase.**

| Method                                | IMGN-C   | IMGN-R   | LSUN-C   | LSUN-R   |
|-:-------------------------------------|-:-:------|-:-:------|-:-:------|-:-:------|
| Plain Softmax                         | 63.9     | 65.3     | 64.2     | 64.7     |
| CROSR [@yoshihashi2019classification] | 72.1     | 73.5     | 72.0     | 74.9     |
| GFROSR [@perera2020generative]        | 75.7     | 79.2     | 75.1     | 80.5     |
| C2AE [@oza2019c2ae]                   | 83.7     | 82.6     | 78.3     | 80.1     |
| CGDL [@sun2020conditional]            | 84.0     | 83.2     | 80.6     | 81.2     |
| PROSER [@zhou2021learning]            | 84.9     | 82.4     | 86.7     | 85.6     |
| **CSSR**                              | **92.9** | **90.9** | **94.1** | **93.5** |
| **RCSSR**                             | **93.3** | **91.5** | **94.0** | **94.0** |

#### OOD Detection

In this section, we followed the experimental settings of Chen [@chen2021adversarial] to compare with methods in an OOD detection setting. We considered two challenging
pairs of OOD detection benchmarks [@hendrycks2016a], including three common datasets: CIFAR10, CIFAR100, and SVHN. The models were trained on CIFAR10, whereas CIFAR100
and SVHN served as the near OOD and far OOD datasets, respectively, during the test phase. Note that the overlapping categories were removed from CIFAR100. In addition to
AUROC, we used several other evaluation metrics, following Chen [@chen2021adversarial]:

  * **Detection accuracy (DTACC).** This metric represents the maximum known/unknown classification accuracy over all possible thresholds. In calculating accuracy, the
  positive and negative samples were assumed to have equal probability to appear in the test set.
  * **Area under the precision-recall curve (AUPR).** The curve plots precision, $TP/(TP+FP)$, against recall, $TP/(TP+FN)$, with a varying threshold, where $TP, FP$,
  and $FN$ denote true positive, false positive, and false negative, respectively. AUPR is further calculated as AUIN and AUOUT, where in- and out-distribution samples
  are set as positive, respectively.

As presented in Table 3, we compared the results with those from ARPL [@chen2021adversarial]. For the near OOD dataset, CSSR performs comparable to ARPL, and is
significantly better than primary prototype point-based open set recognition (GCPL), while RCSSR outperforms ARPL by an increment of 2.3%. For the far OOD dataset,
we observed that CSSR and RCSSR have similar performance; both of them outperformed traditional prototype-like methods by a large margin.

**Table 3: OOD Detection comparison.**

| Method                      | DTACC (CIFAR100) | AUROC (CIFAR100) | AUIN (CIFAR100) | AUOUT (CIFAR100) | DTACC (SVHN) | AUROC (SVHN) | AUIN (SVHN) | AUOUT (SVHN) |
|-:---------------------------|-:-:--------------|-:-:--------------|-:-:-------------|-:-:--------------|-:-:----------|-:-:----------|-:-:---------|-:-:----------|
| SoftMax                     | 79.8             | 86.3             | 88.4            | 82.5             | 86.4         | 90.6         | 88.3        | 93.6         |
| GCPL [@prototype]           | 80.2             | 86.4             | 86.6            | 84.1             | 86.1         | 91.3         | 86.6        | 94.8         |
| RPL [@chen2020learning]     | 80.6             | 87.1             | 88.8            | 83.8             | 87.1         | 92.0         | 89.6        | 95.1         |
| ARPL [@chen2021adversarial] | 83.4             | 90.3             | 91.1            | 88.4             | 91.6         | 96.6         | 94.8        | 98.0         |
| CSI [@tack2020csi]          | 84.4             | 91.6             | 92.5            | 90.0             | 92.8         | 97.9         | 96.2        | 99.0         |
| OpenGAN [@kong2021opengan]  | 84.2             | 89.7             | 87.7            | 89.6             | 92.1         | 95.9         | 93.4        | 97.1         |
| **CSSR**                    | **83.8**         | **92.1**         | **89.4**        | **89.3**         | **95.7**     | **99.1**     | **98.2**    | **99.6**     |
| **RCSSR**                   | **85.3**         | **92.3**         | **92.9**        | **91.0**         | **95.7**     | **99.1**     | **98.3**    | **99.6**     |


### Ablation Study

The contributions from different components and score functions of CSSR are analyzed in this section. We first compare various architectures for the model.

**Datasets:** We trained the models on CIFAR10. For experiments on CIFAR10, we used all 10 classes in CIFAR10 as known classes, and then, tested on SVHN, LSUN-Resize,
ImageNet-Resize, LSUN-Fix (LSUN-F), and ImageNet-Fix (IMGN-F). LSUN-Fix/ImageNet-Fix contains randomly sampled and resized images from LSUN/ImageNet produced by Tack [@tack2020csi],
and the two datasets are more challenging than the original version released by Liang [@liang2018enhancing].

**Ablation Terms.** 
(1) **Classification Layer:** We compared traditional classification models with plain linear classification layers as baselines, and we kept the backbone and hyperparameters fixed for a fair comparison. 
(2) **Classification Strategy:** We used the proposed pixelwise prediction strategy (pixelwise SoftMax; then average pooling, namely SM-AP) or plain prediction strategy
(average pooling; then SoftMax, namely AP-SM). The pixelwise prediction strategy affects both training and testing. 
(3) **Reconstruction Error Measurement:** We measured reconstruction errors with MSE or MAE (by default) for CSSR.

**Table 4: Ablation study on model components.**

| Method       | Close Acc | SVHN     | LSUN-R   | IMGN-R   | LSUN-F   | IMGN-F   | Average  |
|-:------------|-:-:-------|-:-:------|-:-:------|-:-:------|-:-:------|-:-:------|-:-:------|
| Linear       | 96.77     | 97.0     | 95.3     | 94.2     | 92.3     | 93.2     | 94.4     |
| Linear SM-AP | 96.96     | 96.8     | 95.7     | 94.8     | 92.9     | 93.4     | 94.7     |
| CSSR AP-SM   | 96.69     | 98.9     | 98.5     | 97.1     | 89.7     | 89.5     | 94.7     |
| CSSR MSE     | 96.85     | 98.9     | 98.5     | 97.3     | 95.4     | 94.4     | 96.9     |
| **CSSR**     | **96.86** | **99.1** | **98.8** | **97.5** | **96.2** | **95.3** | **97.4** |
| RCSSR AP-SM  | 96.84     | 97.4     | 97.1     | 95.5     | 88.1     | 89.2     | 93.5     |
| RCSSR MSE    | 96.82     | 98.7     | 97.8     | 96.5     | 92.0     | 91.1     | 95.2     |
| **RCSSR**    | **97.02** | **99.1** | **99.1** | **98.1** | **96.0** | **95.0** | **97.3** |

The results are shown in Table 4. The table shows the following: (1) As a non-linear classification layer, CSSR slightly improves closed set performance.
  (2) Pixelwise classification slightly improves closed set classification performance, while largely improving the performance of unknown detection.
  (3) Using a distance measure of MAE generally outperforms MSE, demonstrating that MSE is a good choice for detecting unknown samples.

Next, the effect of different score functions is analyzed. We compared them by fixing trained CSSR and specifying different score functions for decision making.
ImageNet30 (a subset of ImageNet introduced by Hendrycks [@hendrycks2019using]) was utilized in this experiment, where 10 classes were sampled as known and the
remaining 20 classes as unknown. The class split was kept the same for all experiments, where the top 10 classes in alphabetical order were selected as known
classes for simplicity. To adapt to ImageNet-30, which has higher image resolution, RandomCrop was replaced by RandomResizedCrop, following standard data augmentations
in training ImageNet. A plain ResNet18 with linear classification layer was considered as the baseline. 

Six different score functions were compared: relative reconstruction

	error (RRE, $\frac{d(\mathbf{z},\mathcal{A}_c)}{|\mathbf{z}|_1}$),
	feature magnitude (FM, $|\mathbf{z}|_1$), $s_{*1}$ ($s_{p1}$ for CSSR,$s_{r1}$ for RCSSR, and maximum SoftMax probability for baseline),
	$s_2$,
	$s_3$,
	and $s_{all}$.

**Table 5: AUROC comparison on different score functions for ablation models trained on ImageNet-30.**

| Methods     | RRE      | FM       | $s_{*1}$ | $s_2$    | $s_3$    | $s_{all}$ |
|-:-----------|-:-:------|-:-:------|-:-:------|-:-:------|-:-:------|-:-:-------|
| Linear      | -        | 39.3     | 93.5     | 92.5     | 87.1     | -         |
| CSSR AG-SM  | 93.6     | 90.6     | 92.2     | 94.0     | 91.9     | 94.6      |
| CSSR MSE    | 85.1     | 81.9     | 92.2     | 95.1     | 93.9     | 94.7      |
| **CSSR**    | **91.3** | **91.5** | **94.8** | **95.5** | **94.6** | **95.3**  |
| RCSSR AG-SM | 84.5     | 84.7     | 95.1     | 92.4     | 92.1     | 94.7      |
| RCSSR MSE   | 89.7     | 81.3     | 95.0     | 94.2     | 93.7     | 94.6      |
| **RCSSR**   | **86.8** | **90.2** | **95.1** | **94.6** | **94.4** | **95.0**  |

The results are illustrated in Table 5, where we observe the following: (1) The features learned by the plain linear classification layer are less class-related;
the feature magnitude is not sensitive to detect unknown classes, and the representation-based score functions are less discriminative to the unknown classes.
(2) CSSR, which improves representation learning ability, also improves the two representation-based score functions $s_2, s_3$. (3) For $s_{*1}$,
integrating both RRE and FM significantly improves the open set performance for both CSSR and RCSSR. (4) Pixelwise prediction and MAE are good at improving the
quality of the learned representations, and therefore improving the performance of representation-based score functions.

Although the score fusion is not guaranteed to improve all of the individual scores, it approximately maintains the best individual score. We demonstrate that the
three score functions have different performances on different datasets. Retaining the best performance of the three score functions improves the overall performance
across datasets and reduces the variance. We further show how different score functions perform in the unknown detection experiment (Table 1) to demonstrate performance
variation under different datasets. For example, $s_3$ gains an advantage in CIFAR+10 and CIFAR+50, but not in SVHN and TinyImageNet. The fused score, however, is at
least the second best and has minimal standard deviation.

## Further Analysis

### Closed Set Performance

**Table 6: Closed set accuracy comparison.**
| Method                                | Accuracy |
|-:-------------------------------------|-:-:------|
| CROSR [@yoshihashi2019classification] | 94.0     |
| CGDL [@sun2020conditional]            | 91.2     |
| GCPL [@prototype]                     | 93.3     |
| ARPL [@chen2021adversarial]           | 94.0     |
| Our baseline                          | 95.1     |
| **CSSR**                              | **95.3** |
| **RCSSR**                             | **95.6** |


### Performance Against Openness

Openness [@scheirer2013toward], as a measure representing the complexity of the open set task, is defined by:
	$$
	\begin{aligned}
	\text{Openness} = 1 - \sqrt{\frac{2 \times N_{train}}{N_{test} + N_{target}}},
	\end{aligned}
	$$
where $N_{train}$ is the number of known classes seen during training, $N_{test}$ is the number of classes that will be observed during testing,
and $N_{target}$ is the number of classes to be recognized during testing. Using common experimental settings [@sun2020conditional; @zhou2021learning],
we conducted the experiment on CIFAR100, where 15 classes were randomly sampled as known classes. The number of unknown classes varied from 15 to 85,
meaning that the openness varied from 18% to 49%. The recognition performances of 16 classes (15 known classes and 1 unknown) were evaluated by classification accuracy.
CSSR shows good performance with increasing openness, while the performance drops rapidly when using a plain linear classification layer.

### Performance on Large-scale Datasets

To evaluate our model on a large-scale classification task, we conducted experiments on ImageNet-1000 [@russakovsky2015imagenet]. As a more challenging dataset,
ImageNet-1000 includes 1,000 classes with more than one million training images and 50,000 validation images. We adopted ResNet18 as the backbone network. To adapt
to large-scale classification and save parameters, we reduced the dimension of the embedding space for AEs from 64 to 32. The learning rate still started at 0.4,
but dropped by a factor of 10 at the 100 and 150 epoch for sufficient training. Considering the larger and more complex semantic space introduced by ImageNet-1000,
we disabled the representation-based score functions, i.e., $w_2=w_3=0$. The remaining hyperparameters were kept the same as in the experiments on ImageNet30.
In addition to considering the model plain linear classification layer as our baseline, we took the reported results for the original prototype-based method [@prototype]
to make comparisons. Note that the implementation in [@prototype] leveraged ResNet50 as the backbone, which is much stronger than ResNet18 as used in our study.

Two additional metrics are used to evaluate the models more comprehensively: (1) **TNR@TPR95** is the probability that an unknown sample is correctly rejected with 
the true positive rate (TPR) being $95\%$; (2) **open set classification rate (OSCR)**, as defined in [@dhamija2018reducing], was adopted. We denote the score threshold
 by $\delta$. Correct classification rate (CCR) is the fraction of known samples that are correctly classified with unknown detecting scores above the given threshold
 $\delta$. False positive rate (FPR) is the fraction of unknown samples whose unknown detecting scores are greater than threshold $\delta$. The CCR and FPR values under
 different thresholds were reduced to one specific value by taking the area under CCR against the FPR curve.

**Table 7: Results on ImageNet-1000. Performances for both unknown detection and open set recognition are evaluated.**

| Method               | AUROC (Unknown detection) | TNR@TPR95 (Unknown detection) | Macro-F1 (Open set recognition) | OSCR (Open set recognition) |
|-:--------------------|-:-:-----------------------|-:-:---------------------------|-:-:-----------------------------|-:-:-------------------------|
| SoftMax [@prototype] | 79.7                      | -                             | -                               | -                           |
| GCPL [@prototype]    | 82.3                      | -                             | -                               | -                           |
| Our baseline         | 91.0                      | 51.8                          | 40.1                            | 77.4                        |
| **CSSR**             | **93.7**                  | **62.4**                      | **43.0**                        | **78.1**                    |
| **RCSSR**            | **93.1**                  | **58.4**                      | **40.7**                        | **77.6**                    |


The results are shown in Table 7. We can first observe that our implementation for baseline clearly outperforms previous studies in the literature, indicating an
underestimation of the open set performance on large-scale datasets. The proposed CSSR and RCSSR outperform the baseline in detecting unknown classes, especially CSSR.
It is also observed that the improvement in open set recognition for RCSSR appears relatively small compared to the task of unknown detection. This is due to the
degradation of the closed set performance for RCSSR.

**Table 8: Additional ImageNet-1000 Unknown Detection Metrics.**

| Method                      | AUROC    | TNR@TPR95 | DTACC    |
|-:---------------------------|-:-:------|-:-:-------|-:-:------|
| SoftMax                     | 87.2     | 41.2      | 79.1     |
| ARPL [@chen2021adversarial] | 88.8     | 50.6      | 80.2     |
| OpenGAN [@kong2021opengan]  | 89.3     | 32.0      | 84.3     |
| **CSSR**                    | **93.8** | **70.7**  | **86.5** |
| **RCSSR**                   | **94.5** | **73.8**  | **87.2** |


**Table 9: Training Time Comparison.**

| Method  | Training Strategy | Time |
|-:-------|-:-----------------|-:----|
| Plain   | From Scratch      | 140h |
| CSSR    | From Scratch      | 225h |
| ARPL    | Fix Backbone      | +6h  |
| OpenGAN | Fix Backbone      | +1h  |
| CSSR    | Fix Backbone      | +8h  |

*(Note: Figure references for memory/epoch time and score ablation have been preserved as structural placeholders per the original text).*

### Analysis of Failures

To demonstrate the difference between CSSR and plain classification models with a linear classification layer, the models trained in the ImageNet30 experiment (Table 5)
were chosen. Then, for both CSSR and plain linear, we picked the worst recognized known and unknown samples for visualization. Specifically, we selected the six samples
with the lowest detection scores in category flight (known during training), and six samples with the highest scores among unknown samples, representing the worst failures.
We further constrained the unknown samples to be predicted as category flight for better comparison. 

For the failures on the known samples, CSSR mainly focuses on distant images, where the target object is relatively small. However, the plain model also fails on occasions 
there the plane is too close. As the known classes are modeled with intra-diversity allowed for CSSR, visual variants have less influence on the overall recognition. For the
failures on the unknown samples, the sailboats cause the most confusion, which might be because of the similar backgrounds and textures. However, in the plain model, the
tanks are not expected to be confused, and such mistakes can lead to severe issues in real-world applications.

***
