# From Open Set to Closed Set: Counting Objects by Spatial Divide-and-Conquer

## 1. Introduction

The task of visual counting in Computer Vision is to infer the number of objects (people, cars, maize tassels, etc.) from an image/video. It has wide applications, such as automatic crowd management [[1](https://www.google.com/search?q=%23ref-1), [2](https://www.google.com/search?q=%23ref-2), [3](https://www.google.com/search?q=%23ref-3), [4](https://www.google.com/search?q=%23ref-4), [5](https://www.google.com/search?q=%23ref-5)], traffic monitoring [[6](https://www.google.com/search?q=%23ref-6), [7](https://www.google.com/search?q=%23ref-7)], and crop yield estimation [[8](https://www.google.com/search?q=%23ref-8), [9](https://www.google.com/search?q=%23ref-9), [10](https://www.google.com/search?q=%23ref-10)]. Extensive attention has been received in recent years.


*Figure 1: The histogram of count values of $64 \times 64$ local patches on the test set of ShanghaiTech Part_A dataset [[5](https://www.google.com/search?q=%23ref-5)]. The orange curve denotes the relative mean absolute error (rMAE) of CSRNet [[11](https://www.google.com/search?q=%23ref-11)] on local patches.*

Counting is an open-set problem by nature as a count value can range from $0$ to $+\infty$ in theory. It is thus typically modeled in a regression manner. Benefiting from the success of convolutional neural networks (CNNs), state-of-the-art deep counting networks often adopt a multi-branch architecture to enhance the feature robustness to dense regions [[12](https://www.google.com/search?q=%23ref-12), [13](https://www.google.com/search?q=%23ref-13), [5](https://www.google.com/search?q=%23ref-5)].

However, the observed patterns in datasets are limited in practice, which means networks can only learn from a closed set. *Are these counting networks still able to generate accurate predictions when the number of objects is out of the scope of the closed set?* Meanwhile, observed local counts exhibit a long-tailed distribution shown in **Figure 1**. Extremely dense patches are rare while sparse patches take up the majority. As can be observed, the relative mean absolute error (rMAE) increases significantly with increased local density. *Is it necessary to set the working range of CNN-based regressors to the maximum count value observed, even when a majority of samples are sparse such that the regressor works poorly in this range?*

In fact, counting has a unique property—it is **spatially decomposable**. The above problem can be largely alleviated with the idea of **Spatial Divide-and-Conquer (S-DC)**. Suppose that a network has been trained to accurately predict a closed set of counts, say $0 \sim 20$. When facing an image with extremely dense objects, one can keep dividing the image into sub-images until all sub-region counts are less than $20$. Then the network can accurately count these sub-images and sum over all local counts to obtain the global image count.

**Figure 2** graphically depicts the idea of S-DC. A follow-up question is how to spatially divide the count. A naive way is to upsample the input image, divide it into sub-images, and process sub-images with the same network. This way, however, is likely to blur the image and lead to exponentially increased computation cost and memory consumption when repeatedly extracting the feature map. Inspired by RoI pooling [[14](https://www.google.com/search?q=%23ref-14)], we show that it is feasible to achieve S-DC on the feature map, as conceptually illustrated in **Figure 3**. By decoding and upsampling the feature map, the later prediction layers can focus on the feature of local areas and predict sub-region counts accordingly.


*Figure 2: An illustration of spatial divisions. Suppose that the closed set of counts is $[0, 20]$. In this example, dividing the image once is inadequate to ensure that all sub-region counts are within the closed set. For the top-left sub-region, a further division is needed.*


*Figure 3: Spatial divisions on the input image (left) and the feature map (right). Spatially dividing the input image is straightforward. The image is upsampled and fed to the same network to infer counts of local areas. S-DC on the feature map avoids redundant computations and is achieved by upsampling, decoding, and dividing the feature map of high resolution.*

To realize the above idea, we propose a simple but effective **Spatial Divide-and-Conquer Network (S-DCNet)**. S-DCNet learns from a closed set of count values but is able to generalize to open-set scenarios. Specifically, S-DCNet adopts a VGG16 [[15](https://www.google.com/search?q=%23ref-15)]-based encoder and an UNet [[16](https://www.google.com/search?q=%23ref-16)]-like decoder to generate multi-resolution feature maps. All feature maps share the same counting predictor.

Inspired by [[17](https://www.google.com/search?q=%23ref-17)], in contrast to conventional density map regression, we discretize continuous count values into a set of intervals and design the counting predictor to be a classifier. Further, a division decider is designed to decide which sub-region should be divided and to merge different levels of sub-region counts into the global image count.

We show through a controlled toy experiment that, even given a closed training set, S-DCNet effectively generalizes to the open test set. The effectiveness of S-DCNet is further demonstrated on three crowd counting datasets (ShanghaiTech [[5](https://www.google.com/search?q=%23ref-5)], UCF_CC_50 [[1](https://www.google.com/search?q=%23ref-1)], and UCF-QNRF [[2](https://www.google.com/search?q=%23ref-2)]), a vehicle counting dataset (TRANCOS [[6](https://www.google.com/search?q=%23ref-6)]), and a plant counting dataset (MTC [[10](https://www.google.com/search?q=%23ref-10)]). Results show that S-DCNet indicates a clear advantage over other competitors and sets a new state-of-the-art across five datasets.

The main contribution of this work is that we propose to transform open-set counting into a closed-set problem. We show through extensive experiments that a model learned in a closed set can effectively generalize to the open set with the idea of S-DC.

---

## 2. Related Work

Current CNN-based counting approaches are mainly built upon the framework of local regression. According to their regression targets, they can be categorized into two categories: **density map regression** and **local count regression**.

### 2.1 Density Map Regression

The concept of density map was introduced in [[18](https://www.google.com/search?q=%23ref-18)]. The density map contains the spatial distribution of objects, thus can be smoothly regressed. Zhang et al. [[4](https://www.google.com/search?q=%23ref-4)] first adopted a CNN to regress local density maps. Then almost all subsequent counting networks followed this idea. Among them, a typical network architecture is multi-branch:

* **MCNN** [[5](https://www.google.com/search?q=%23ref-5)] and **Switching-CNN** [[12](https://www.google.com/search?q=%23ref-12)] used three columns of CNNs with varying receptive fields to depict objects of different scales.
* **SANet** [[13](https://www.google.com/search?q=%23ref-13)] adopted Inception [[19](https://www.google.com/search?q=%23ref-19)]-like modules to integrate extra branches.
* **CP-CNN** [[20](https://www.google.com/search?q=%23ref-20)] added two extra density-level prediction branches to combine global and local contextual information.
* **ACSCP** [[21](https://www.google.com/search?q=%23ref-21)] inserted a child branch to match cross-scale consistency and an adversarial branch to attenuate the blurring effect of the density map.
* **ic-CNN** [[22](https://www.google.com/search?q=%23ref-22)] incorporated two branches to generate high-quality density maps in a coarse-to-fine manner.
* **IG-CNN** [[23](https://www.google.com/search?q=%23ref-23)] and **D-ConvNet** [[24](https://www.google.com/search?q=%23ref-24)] drew inspiration from ensemble learning and trained a series of networks or regressors to tackle different scenes.
* **DecideNet** [[25](https://www.google.com/search?q=%23ref-25)] attempted to selectively fuse the results of density map estimation and object detection.
* **Idrees et al.** [[2](https://www.google.com/search?q=%23ref-2)] employed a composition loss and simultaneously solved several counting-related tasks to assist counting.
* **CSRNet** [[11](https://www.google.com/search?q=%23ref-11)] benefited from dilated convolution which effectively expanded the receptive field to capture contextual information.

Existing deep counting networks aim to generate high-quality density maps. However, density maps are actually in the open set as well.

### 2.2 Local Count Regression

Local count regression directly predicts count values of local image patches. This idea first appeared in [[26](https://www.google.com/search?q=%23ref-26)] where a multi-output regression model was used to regress region-wise local counts simultaneously. Count-ception [[27](https://www.google.com/search?q=%23ref-27)] and TasselNet [[10](https://www.google.com/search?q=%23ref-10)] introduced such an idea into deep counting. Local patches were first densely sampled in a sliding-window manner with overlaps, and a local count was assigned to each patch by the network. Inferred redundant local counts were normalized and fused to the global count.

Stahl et al. [[28](https://www.google.com/search?q=%23ref-28)] regressed counts for object proposals generated by Selective Search [[29](https://www.google.com/search?q=%23ref-29)] and combined local counts using an inclusion-exclusion principle. Inspired by subitizing (the human ability to quickly count a few objects at a glance), Chattopadhyay et al. [[30](https://www.google.com/search?q=%23ref-30)] focused on counting objects in everyday scenes.

While some of the above methods [[30](https://www.google.com/search?q=%23ref-30), [28](https://www.google.com/search?q=%23ref-28)] also leverage the idea of spatial divisions, they still regress open-set counts. Since only finite local patterns (a closed set) can be observed, new scenes in reality have a high probability of including objects out of the observed range (an open set). In this paper, we show that a counting network is able to learn from a closed set with a certain range of counts (e.g., $0 \sim 20$) and then generalize to an open set (including counts $> 20$) via S-DC.

### 2.3 Beyond Naive Regression

Some literature suggests that regression can be reformulated as an ordinal regression problem or a classification problem, which enhances performance and benefits optimization [[31](https://www.google.com/search?q=%23ref-31), [32](https://www.google.com/search?q=%23ref-32), [17](https://www.google.com/search?q=%23ref-17), [33](https://www.google.com/search?q=%23ref-33)]. Li et al. [[17](https://www.google.com/search?q=%23ref-17)] showed that directly reformulating regression to classification was effective. Since count values share similar properties with age and depth, S-DCNet follows [[17](https://www.google.com/search?q=%23ref-17)] to discretize local counts and classify count intervals.

---

## 3. Spatial Divide-and-Conquer Network

### 3.1 From Quantity to Interval

Instead of regressing an open set of count values, we discretize local counts and classify count intervals. Specifically, we define an interval partition of $[0, +\infty)$ as:

$$\{0\}, (0, C_1], (C_1, C_2], \dots, (C_{M-1}, C_M], \text{ and } (C_M, +\infty)$$

These $M+1$ sub-intervals are labeled as classes $0$ through $M$, respectively. For example, if a count value lies within $(C_1, C_2]$, it is assigned to class $1$. In practice, $C_M$ should not exceed the maximum local count observed in the training set.

The median of each sub-interval is adopted when recovering the count from the interval. For the last sub-interval $(C_M, +\infty)$, $C_M$ is used as the count value. Adopting $C_M$ for the last class introduces a systematic error, but this error is mitigated via S-DC.


*Figure 4: The overall architecture of Spatial Divide-and-Conquer Network (S-DCNet).*

#### Table 1: Architecture of Classifier and Division Decider

*Note: $\text{AvgPool}$ denotes average pooling. Convolutional layers are defined in format: $\text{Conv } \text{size} \times \text{size}, \text{output channel}, \text{stride}$. Each convolutional layer is followed by a ReLU function except the final layer. A Sigmoid function is employed at the end of the division decider.*

| Classifier Module | Division Decider Module |
| --- | --- |
| $2 \times 2 \text{ AvgPool, stride } 2$ | $2 \times 2 \text{ AvgPool, stride } 2$ |
| $1 \times 1 \text{ Conv}, 512, \text{stride } 1$ | $1 \times 1 \text{ Conv}, 512, \text{stride } 1$ |
| $1 \times 1 \text{ Conv}, \text{class\_num}, \text{stride } 1$ | $1 \times 1 \text{ Conv}, 1, \text{stride } 1$ |
| — | $\text{Sigmoid}$ |

---

### 3.2 Single-Stage Spatial Divide-and-Conquer

S-DCNet includes a VGG16 [[15](https://www.google.com/search?q=%23ref-15)] feature encoder, an UNet [[16](https://www.google.com/search?q=%23ref-16)]-like decoder, a count-interval classifier, and a division decider (**Table 1**).

1. **Feature Extraction:** Given an input patch of size $64 \times 64$, the feature encoder outputs $F_0$ (from Conv5 layer) at $\frac{1}{32}$ resolution of the input image.
2. **Initial Prediction ($C_0$):** The classifier predicts the count interval class $CLS_0$ based on $F_0$. The local count $C_0$ for the $64 \times 64$ patch is recovered from $CLS_0$.
3. **Feature Upsampling & First-Stage Division ($C_1$):** $F_0$ is upsampled by $\times 2$ in an UNet-like manner to produce $F_1$. The shared classifier processes $F_1$ to predict division sub-counts $C_1 \in \mathbb{R}^{2 \times 2}$, where each element represents a sub-count for a $32 \times 32$ sub-region.
4. **Division Decider ($W_1$):** The division decider generates a soft division mask $W_1 \in [0, 1]^{2 \times 2}$ conditioned on $F_1$. Here, $w = 0$ means no division is needed (keep $C_0$), while $w = 1$ replaces the initial prediction with the division sub-count in $C_1$.
5. **Merging Results:** $C_0$ is upsampled by $\times 2$ to $\hat{C}_0$ via spatial averaging. The first-stage division output $DIV_1$ is computed as:

$$DIV_1 = (\mathbf{1} - W_1) \circ \operatorname{avg}(C_0) + W_1 \circ C_1$$

where $\mathbf{1}$ is a matrix of ones with the same dimensions as $W_1$, $\circ$ represents the Hadamard product, and $\operatorname{avg}(\cdot)$ is an averaging redistribution operator that equally divides a count value across a $2 \times 2$ region.

---

### 3.3 Multi-Stage Spatial Divide-and-Conquer

S-DCNet can perform multi-stage S-DC recursively by further decoding and dividing feature maps:

$$DIV_i = (\mathbf{1} - W_i) \circ \operatorname{avg}(DIV_{i-1}) + W_i \circ C_i \quad \text{for } i \ge 2$$

#### Loss Function

S-DCNet is trained using multi-task supervision combining cross-entropy classification losses $L_C^i$ and an $\ell_1$ regression loss $L_R^N$ for the final division output $DIV_N$ ($N$ represents the maximum division stage):

$$L = \sum_{i=0}^N L_C^i + L_R^N$$

The regression loss $L_R^N$ provides an implicit supervision signal for optimizing the soft division masks $W_i$.

#### Algorithm 1: Multi-Stage S-DCNet Inference

```text
Input  : Image I
Output : Global count C

1. Extract feature map F_0 from image I
2. Predict CLS_0 given F_0 with classifier; recover count C_0
3. Initialize DIV_0 = C_0
4. For i = 1 to N do:
     a. Decode F_{i-1} to obtain high-resolution feature map F_i
     b. Predict CLS_i and division mask W_i given F_i
     c. Recover division count map C_i from CLS_i
     d. Update DIV_i = (1 - W_i) * avg(DIV_{i-1}) + W_i * C_i
5. Integrate DIV_N over spatial dimensions to obtain total image count C
Return C

```

---

## 4. Open Set or Closed Set? A Toy-Level Justification

To explore whether a closed-set model can generalize to open-set scenarios, we evaluate models on a synthetic cell counting dataset inspired by [[18](https://www.google.com/search?q=%23ref-18)].

### 4.1 Synthesized Dataset

* **Training Set (Closed Set):** $500$ images of size $256 \times 256$, where $64 \times 64$ sub-regions contain only $0 \sim 10$ cells.
* **Testing Set (Open Set):** $500$ images with sub-region counts evenly distributed in $[0, 20]$.

### 4.2 Baselines & Setup

1. **Regression Baseline:** VGG16 backbone with single-channel output regressing open-set counts via $\ell_1$ loss.
2. **Classification Baseline:** VGG16 backbone using the S-DCNet interval classifier without S-DC.
3. **S-DCNet:** Proposed architecture learning from the closed set ($0 \sim 10$) and expanding via S-DC.

An interval step of $0.5$ is used for discretization, yielding sub-intervals $\{0\}, (0, 0.5], (0.5, 1], \dots, (9.5, 10]$, and $(10, +\infty)$.


*Figure 5: Toy-level justification. (a) Sample images from the synthetic dataset. (b) Mean Absolute Error (MAE) vs. $64 \times 64$ sub-region count. S-DCNet(N) denotes an N-stage S-DCNet.*

### 4.3 Results & Observations

As shown in **Figure 5(b)**, both standard regression and classification baselines perform well within the closed set $[0, 10]$, but their error increases rapidly when local counts exceed $10$. In contrast, S-DCNet maintains accurate predictions on open-set counts ($> 10$), confirming the efficacy of spatial divide-and-conquer.

---

## 5. Experiments on Real-World Datasets

### 5.1 Implementation Details

#### Interval Partition Strategies

1. **One-Linear Partition:** Uses a fixed step of $0.5$: $\{0\}, (0, 0.5], (0.5, 1], \dots, (C_{max}-0.5, C_{max}]$, and $(C_{max}, +\infty)$.
2. **Two-Linear Partition:** Applies a fine-grained step of $0.05$ for the low-count interval $(0, 0.5]$ to capture subtle non-zero object presences, and a $0.5$ step for higher counts.

#### Data Augmentation & Training

* **Augmentation:** $9$ sub-images of $\frac{1}{4}$ resolution are cropped per image (4 corners + 5 random crops) along with random scaling and horizontal flipping. For UCF-QNRF [[2](https://www.google.com/search?q=%23ref-2)], crops are sized $224 \times 224$.
* **Training Setup:** Optimized with standard SGD (initial learning rate $0.001$, dropped by $\times 10$ on plateau). Batch size is set to $1$ (except UCF-QNRF, where batch size is $16$).

---

### 5.2 Ablation Study on ShanghaiTech Part_A

#### Robustness to $C_{max}$


*Figure 6: Influence of $C_{max}$ on ShanghaiTech Part_A. Quantiles represent upper bounds of observed training counts. 'VGG16 Encoder' represents the baseline without S-DC.*

As shown in **Figure 6**, the pure classification baseline degrades severely as $C_{max}$ decreases. S-DCNet maintains stable low MAE across various quantile selections of $C_{max}$, proving that spatial divide-and-conquer effectively mitigates interval truncation errors.

#### Impact of Division Stages

#### Table 2: Performance across different division stages on ShanghaiTech Part_A

| Division Stages | MAE | MSE |
| --- | --- | --- |
| 0 | 76.0 | 142.5 |
| 1 | 62.2 | 103.4 |
| **2** | **58.3** | **95.0** |
| 3 | 60.1 | 99.8 |
| 4 | 61.9 | 107.2 |

Two-stage division achieves the optimal balance between spatial refinement and accuracy (**Table 2**).

#### Comparison with Baseline Formulations

#### Table 3: Performance of different counting paradigms

| Method | MAE | MSE |
| --- | --- | --- |
| Classification (No S-DC) | 77.4 | 149.3 |
| Regression (No S-DC) | 68.9 | 112.1 |
| Open-set Regression + S-DC | 66.6 | 107.9 |
| Closed-set Regression + S-DC | 64.7 | 105.7 |
| **S-DCNet (2-Stage)** | **58.3** | **95.0** |

#### Table 4: Loss Component Ablation

| Classification Losses $\sum_{i=0}^2 L_C^i$ | Regression Loss $L_R^2$ | MAE | MSE |
| --- | --- | --- | --- |
| ✓ | ✗ | 301.4 | 396.9 |
| ✗ | ✓ | 88.4 | 128.8 |
| **✓** | **✓** | **58.3** | **95.0** |

Combining classification losses with the overall division regression loss $L_R^2$ is essential to properly guide both the count predictor and the division decider (**Table 4**).


*Figure 7: Counting errors of $64 \times 64$ local patches on the test set of ShanghaiTech Part_A [[5](https://www.google.com/search?q=%23ref-5)]. $C_0$, $C_1$, and $C_2$ denote single-branch predictions from feature maps $F_0$, $F_1$, and $F_2$.*

---

## References

[1] H. Idrees, I. Saleemi, C. Seibert, and M. Shah. "Multi-source multi-scale counting in extremely dense crowd images." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2013.

[2] H. Idrees et al. "Composition loss for counting, density map estimation and localization in dense crowds." In *European Conference on Computer Vision (ECCV)*, 2018.

[3] S. Bai et al. "Finding tiny faces in the crowd with blobs." In *European Conference on Computer Vision (ECCV)*, 2018.

[4] C. Zhang et al. "Cross-scene crowd counting via deep convolutional neural networks." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2015.

[5] Y. Zhang et al. "Single-image crowd counting via multi-column convolutional neural network." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2016.

[6] R. Guerrero-Gómez-Olmedo et al. "Extremely overlapping vehicle counting." In *Iberian Conference on Pattern Recognition and Image Analysis*, 2015.

[7] C. C. Loy et al. "From open-set to closed-set: Counting objects by spatial divide-and-conquer." *arXiv preprint arXiv:1908.06473*, 2019.

[8] J. A. Fernandez-Gallego et al. "Low-cost assessment of wheat grain yield using source images." *Plant Methods*, 2018.

[9] M. V. Giuffrida et al. "Learning to count leaves in rosette plants." In *BMVC Workshop*, 2015.

[10] H. Lu et al. "TasselNet: counting maize tassels in the wild via local visual information aggregation." *Plant Methods*, 2017.

[11] Y. Li et al. "CSRNet: Dilated convolutional neural networks for understanding the highly congested scenes." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2018.

[12] D. Babu Sam et al. "Switching convolutional neural network for crowd counting." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2017.

[13] X. Cao et al. "Scale-aggregation network for accurate and compact crowd counting." In *European Conference on Computer Vision (ECCV)*, 2018.

[14] R. Girshick. "Fast R-CNN." In *IEEE International Conference on Computer Vision (ICCV)*, 2015.

[15] K. Simonyan and A. Zisserman. "Very deep convolutional networks for large-scale image recognition." *arXiv preprint arXiv:1409.1556*, 2014.

[16] O. Ronneberger et al. "U-Net: Convolutional networks for biomedical image segmentation." In *MICCAI*, 2015.

[17] S. Li et al. "Deep localization-based crowd counting." *arXiv preprint arXiv:1808.08182*, 2018.

[18] V. Lempitsky and A. Zisserman. "Learning to count objects in images." In *Advances in Neural Information Processing Systems (NeurIPS)*, 2010.

[19] C. Szegedy et al. "Going deeper with convolutions." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2015.

[20] V. A. Sindagi and V. M. Patel. "Generating high-quality crowd density maps using contextual pyramid CNNs." In *IEEE International Conference on Computer Vision (ICCV)*, 2017.

[21] Z. Shen et al. "Adversarial cross-scale consistency for crowd counting." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2018.

[22] V. A. Sindagi and V. M. Patel. "Inverse-consistent deep networks for crowd counting." In *European Conference on Computer Vision (ECCV)*, 2018.

[23] D. Babu Sam et al. "Divide and grow: Capturing huge diversity in crowd images with IG-CNN." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2018.

[24] L. Liu et al. "DecideNet: Counting varying density crowds through attention." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2018.

[25] J. Zhou et al. "Deep negative correlation classification for crowd counting." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2018.

[26] K. Chen et al. "Feature mining for localized crowd counting." *European Conference on Computer Vision (ECCV)*, 2012.

[27] J. P. Cohen et al. "Count-ception: Counting by fully convolutional redundant counting." In *IEEE International Conference on Computer Vision Workshops (ICCVW)*, 2017.

[28] T. Stahl et al. "Divide and count: Using object proposals for object counting in images." *IEEE Transactions on Image Processing*, 2019.

[29] J. R. Uijlings et al. "Selective search for object recognition." *International Journal of Computer Vision (IJCV)*, 2013.

[30] P. Chattopadhyay et al. "Counting everyday objects in everyday scenes." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2017.

[31] K. Chen et al. "Cumulative attribute space for ordinal regression." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2013.

[32] H. Fu et al. "Deep ordinal regression network for monetary depth estimation." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2018.

[33] Z. Niu et al. "Ordinal regression with multiple output CNN for age estimation." In *IEEE Conference on Computer Vision and Pattern Recognition (CVPR)*, 2016.