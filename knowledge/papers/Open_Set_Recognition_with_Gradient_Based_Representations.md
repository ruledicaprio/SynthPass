# Open-Set Recognition with Gradient-Based Representations

**Jinsol Lee** and **Ghassan AlRegib**

School of Electrical and Computer Engineering,  
Georgia Institute of Technology, Atlanta, GA, 30332-0250  
{jinsol.lee, alregib}@gatech.edu

---

## Abstract

Neural networks for image classification tasks assume that any given image during inference belongs to one of the training classes. This closed-set assumption is challenged in real-world applications where models may encounter inputs of unknown classes. Open-set recognition aims to solve this problem by rejecting unknown classes while classifying known classes correctly. In this paper, we propose to utilize gradient-based representations obtained from a known classifier to train an unknown detector with instances of known classes only. Gradients correspond to the amount of model updates required to properly represent a given sample, which we exploit to understand the model's capability to characterize inputs with its learned features. Our approach can be utilized with any classifier trained in a supervised manner on known classes without the need to model the distribution of unknown samples explicitly. We show that our gradient-based approach outperforms state-of-the-art methods by up to 11.6% in open-set classification.

**Index Terms** — gradients, open-set recognition, unknown detection, open-set classification, out-of-distribution.

---

## 1. Introduction

Despite the significant advancement in many applications of deep neural networks, they are known to be prone to failure when deployed in real-world environments as they often encounter data that diverges from training conditions [1, 2]. They rely heavily on the implicit closed-world assumption that any given input during inference belongs to one or more of the classes in training data. Limited to the knowns defined by training set, neural networks classify any input images to be among the known classes, even if given inputs are significantly different from training data. In addition, neural networks tend to make overconfident predictions even for the unfamiliar inputs [3, 4], making it more challenging to distinguish the unknowns from the knowns. These types of behaviors of neural networks can have serious consequences when utilized in safety-critical applications, such as autonomous vehicles and medical diagnostics.

Open-set recognition tackles this problem by removing the closed-world assumption. Instead, an open-set classifier assumes that testing samples may come from any class, even unknown during the model training. Most approaches in the literature can be divided into two categories: discriminative models and generative models. Discriminative modeling approaches [5, 6, 7, 8, 9] aim to learn the distribution of known samples to distinguish between the known classes for classification as well as between the known and unknown classes for unknown detection. Generative modeling approaches [10, 11] seek to synthesize samples of unknown classes to help distinguish those of known classes. However, almost all existing approaches are limited to learned features, which may not be sufficient to capture the abnormality in testing samples of unknown classes.

In this work, we propose to utilize gradient-based representations for open-set recognition. As an extension to our previous work [12], we further validate the concept of confounding labels to generate gradient-based representations to differentiate the inputs that a model is familiar with from those that are considered unknown. Rather than relying solely on the learned features from a model, we utilize gradients to gain insights regarding the amount of adjustments to its parameters necessary to properly represent given inputs. We empirically show that the obtained representations can be utilized in an open-set recognition setting where no sample of unknown classes is available during training to capture the distinction between the knowns and the unknowns.

---

## 2. Related Work

### 2.1. Open-Set Recognition

Open-set recognition (OSR) aims to detect the unknowns as well as to correctly classify the knowns. Scheirer et al. [5] proposed a Support Vector Machine-based approach to add an extra hyperplane in parallel to the originally obtained hyperplanes for known classes to differentiate the unknown classes. Bendale and Boult [6] proposed to replace the Softmax layer with the OpenMax layer to obtain the class probabilities of the unknowns. Ge et al. [10] and Neal et al. [11] proposed to utilize generative networks to create synthetic samples of the unknowns and train open-set classifiers with them as an additional class. Yoshihashi et al. [7] utilized latent representations obtained from an hierarchical reconstruction network for robust unknown detection. Oza and Patel [8] employed class-conditioned auto-encoders with a novel training and testing setup. Sun et al. [9] proposed to learn conditional Gaussian distributions for known classification and unknown detection. However, almost all existing approaches rely on features learned from the knowns to characterize the unknowns.

### 2.2. Gradients

Gradient-based optimization techniques [13] have been at the core of numerous large-scale machine learning applications. Apart from their original utility as a tool to search for a converged solution, gradients have been utilized for various purposes, including visualization [14, 15, 16] and adversarial attack generation [3, 17]. Gradients have also been explored to obtain effective representations [18, 19, 20, 21, 12] for many applications including image quality and saliency estimation, and out-of-distribution/anomaly/novelty detection. However, the effectiveness of gradient-based representations has not been fully explored in the application of open-set recognition.

---

## 3. Open-Set Recognition with Gradient-Based Representations

In this section, we introduce our framework for open-set recognition with gradient-based representations. We explain the setups to train and test relevant classifiers and unknown detectors, and we validate the effectiveness of gradient-based representations in open-set recognition settings.

### 3.1. Proposed Open-Set Recognition Framework

In our previous work [12], we introduced the framework to obtain gradient-based representations with confounding labels for anomalous sample detection. We defined a confounding label as a label that is different from ordinary labels on which a model is trained. Our intuition was that gradients correspond to the amount of change a model requires to properly represent a given sample. By introducing an unseen class label to the model with pre-defined representation space, the required model updates captured in gradients would be pertinent to mapping its relevant features to the new class if the model is familiar with the given input. If the input is not within the scope of the model, however, updates will be necessary for feature extraction and mapping, leading to a larger total amount of updates.

Open-set recognition is a natural extension to our previous work by utilizing the gradient representations obtained from a classifier to train an unknown detector to reject the unknowns while classifying the knowns as the classifier was originally intended. We introduce our open-set recognition framework with gradient-based representations in Fig. 1. Given a trained classification network $f(\theta)$ and an input image $x$, the network produces an output $f(x; \theta)$. Binary cross entropy loss is computed between the model output and a confounding label $y_c$, which is a vector of length $N$ with $n$ number of 1's where $N$ is the number of classes in training and $n \in \{0, \dots, N\} \setminus \{1\}$. The loss is backpropagated to generate gradients $\nabla J(\theta; x, y_c)$, and gradient-based representation is formed by concatenating the magnitudes of gradients from every parameter set in the model as the following:

$$
\left[ \| \nabla J_{\theta_0}(\theta; x, y_c) \|_2^2, \cdots, \| \nabla J_{\theta_{P-1}}(\theta; x, y_c) \|_2^2 \right], \tag{1}
$$

where $P$ is the number of parameter sets in a given network.

Based on the decision of the unknown detector $d$, the final classification is determined: if known, the prediction of the original classifier is preserved ($c \in \{0, N - 1\}$); if unknown, then the model prediction is $N$. Overall, there are $N + 1$ possible options for final prediction.

### 3.2. Closed-Set Training & Open-Set Testing

The proposed method for open-set recognition has two stages: closed-set training and open-set testing. Closed-set includes the knowns, while open-set includes both the knowns and the unknowns. The closed-set training phase is then split into two stages: "closed-set" training and "open-set" training, where we randomly select some of the known classes to be the "unknowns" for training. This data split protocol is described in Fig. 2, and we explain the details of each stage in this section with color coordination for clarity.

**Closed-Set Training.** In training any network for open-set recognition, whether it is a classifier or an unknown detector, we have no access to any samples of the unknowns, $U$, as opposed to the knowns, $K$. Therefore, the training samples of the knowns, $K_{train}$, are split into two groups: "knowns", $K_K$, and "unknowns", $K_U$. First, the "known" samples $\in K_K$ are used to train a "closed-set" classifier. Then gradient-based representations for all samples of the "knowns" and "unknowns" $\in K_K \cup K_U = K_{train}$ can be collected to train and validate an "open-set" unknown detector. In addition, we train a closed-set classifier with the training samples of all knowns $\in K_{train}$ to be utilized in the open-set testing stage.

**Open-Set Testing.** For the testing of the open-set recognition framework, we now utilize the closed-set classifier and the "open-set" unknown detector, trained in the previous stage. First, we input the test samples of both the knowns and the unknowns $\in K_{test} \cup U_{test}$ into the closed-set classifier to collect model predictions as well as gradient-based representations, as described in Sec. 3.1. The gradient-based representations are then passed to the trained "open-set" unknown detector to determine whether the samples may be among the knowns or the unknowns. Then, based on the detector prediction, the classification from the closed-set classifier for each test sample is preserved or replaced with a new class label representing the unknowns.

### 3.3. Effectiveness of Gradient-based Representations

In this section, we demonstrate the effectiveness of gradients in open-set recognition setting. We create mainly two testing scenarios: 1) the unknowns chosen from the same dataset as the knowns; 2) utilizing additional datasets as unknowns. For the first scenario, we employ the random class splits of 6 known classes and 4 unknown classes on CIFAR-10 [22] dataset, widely used for the evaluation of open-set recognition approaches. Among the first 6 classes, we also split them into 4 known classes and 2 unknown classes for further analysis. For the second scenario, we use ImageNet and LSUN (resize and crop) datasets as unknowns, collected by [23] and also conventionally used for open-set recognition. In each case, a ResNet-18 classifier is trained with the training set of the knowns and the distributions of gradient magnitudes are collected on the test sets of the knowns and the unknowns to be visualized for 2 different model parameter sets in Fig. 3. The knowns and the unknowns for each figure are specified in the corresponding caption. It is clear that the gradient magnitudes for the knowns are smaller than the unknowns in every case. Comparing Fig. 3(a) and (b), however, there is a more clear distinction in gradient magnitudes between the knowns and the unknowns when the number of known classes and samples are larger. When the unknowns are drawn from different datasets, as shown in Fig. 3(c), the distributions of the knowns and the unknowns are clearly separated. The gradient magnitude distributions of the knowns in Fig. 3(c) are highlighted in red circles for clarity. In all scenarios, gradients prove to be an effective tool to distinguish the unknowns from the knowns.

---

## 4. Experiments

In this section, we utilize the gradient-based representations obtained with confounding labels for open-set recognition. The performance of open-set recognition methods is evaluated mainly in two aspects: open-set identification and open-set classification. Open-set identification focuses on the detection of the unknowns when utilizing a single dataset to define the knowns and the unknowns. On the other hand, open-set classification focuses on the classification accuracy where the model makes predictions of $N + 1$ possible options, including the unknown drawn from different datasets than the knowns. The experiment setups expand upon the described scenarios in Sec. 3.3, with Fig. 3(a) and (b) concerned with open-set identification within a single dataset, while Fig. 3(b) and (c) with open-set classification. We describe the details of each setup in the corresponding sections. For implementations, ResNet-18 with no pre-training is utilized as a classifier, and a binary classifier of 2 fully-connected layers is used as an unknown detector.

### 4.1. Open-Set Identification

For open-set identification, we employ the widely accepted setup of selecting some classes at random to be used as knowns and the remainder as unknowns. In this scenario, the samples of the unknowns come from the same dataset as the knowns. As described in Sec. 3.3 regarding Fig. 3(a) and (b), we first select 6 known classes $\in K$ and 4 unknown classes $\in U$ from the 10 overall classes of CIFAR-10. Then from $K_{train}$, we select 4 "known" classes $\in K_K$ and 2 "unknown" classes $\in K_U$. With $K_K$ and $K_U$, we train an unknown detector and test it on $\in K_{test} \cup U_{test}$. We repeat for 5 different randomized sets of class splits, and we report the performance of unknown detectors in Table 1. While our approach outperforms the existing methods prior to 2019 by a large margin, the more recent methods outperform our method. This is due to the performance of the unknown detectors trained for the 4 "known" and 2 "unknown" classes. As shown in Fig. 3(a), the distinction between the gradient magnitude distributions is less significant when there are fewer number of "known" and "unknown" classes. The trained unknown detectors with the mentioned 4–2 class split show below 90% accuracy on their validation set, leading to even lower discriminative results when evaluated with the unknown classes.

**Table 1:** Open-set identification results on CIFAR-10 dataset in AUROC. For methods other than the proposed method, we report the experimental results from [7, 9].

| Method | CIFAR-10 |
|:---|:---:|
| Softmax | 0.677 |
| OpenMax [6] | 0.695 |
| G-OpenMax [10] | 0.675 |
| OSRCI [11] | 0.699 |
| C2AE [8] | 0.895 |
| CGDL [9] | **0.903** |
| Ours | 0.838 |

### 4.2. Open-Set Classification

For open-set classification, we utilize CIFAR-10 dataset as the knowns and ImageNet and LSUN (resize and crop) as the unknowns. Specifically, the test set of each dataset being used as the unknowns is added to the test set of CIFAR-10. Each dataset as well as the test set of CIFAR-10 contains 10,000 testing samples, making the known-to-unknown ratio 1:1. As described in Sec. 3.3 regarding Fig. 3(b) and (c), we create 6–4 class split using the train set of CIFAR-10 to train an unknown detector. We repeat for 5 different randomized sets of class splits, and we report the open-set classification accuracy on the combined test sets of the knowns and the unknowns in Table 2. During the open-set identification experiments in the previous section, we noticed that the thresholding values for the output of binary classifiers to achieve the best AUROC scores are higher than the regular 0.5, averaging over 0.9, when the detectors trained on 4–2 class split are evaluated on the combined 6–4 class split. Based on this observation, we fix the thresholding value for unknown detectors to 0.95 during open-set classification testing. Our approach with gradient-based representations outperforms all recent methods by a large margin. The unknown detector shows better performance when the unknowns come from different datasets than the classifier training dataset, similar to the out-of-distribution detection setup in our previous work [12]. The exceptional out-of-distribution detection performance with the gradient-based representations is matched with the performance of the unknown detectors in this work. These results prove that gradients prove to be an effective tool to capture the unknowns from the knowns in open-set recognition setting.

**Table 2:** Open-set classification results on CIFAR-10 dataset with various outliers added to the test set as unknowns. For methods other than the proposed method, we report the experimental results from [7, 9].

| Method | ImageNet-resize | ImageNet-crop | LSUN-resize | LSUN-crop |
|:---|:---:|:---:|:---:|:---:|
| Softmax | 0.653 | 0.639 | 0.647 | 0.642 |
| OpenMax [6] | 0.684 | 0.660 | 0.668 | 0.657 |
| LadderNet+OpenMax [7] | 0.670 | 0.653 | 0.659 | 0.652 |
| DHRNet+OpenMax [7] | 0.675 | 0.655 | 0.664 | 0.656 |
| CROSR [7] | 0.735 | 0.721 | 0.749 | 0.720 |
| C2AE [8] | 0.826 | 0.837 | 0.801 | 0.783 |
| CGDL [9] | 0.832 | 0.840 | 0.812 | 0.806 |
| Ours | **0.842** | **0.912** | **0.882** | **0.922** |

---

## 5. Conclusion

In this paper, we utilized gradient-based representations obtained from a trained classifier with confounding labels to detect samples of the unknowns while preserving model predictions for those detected to be the knowns. We empirically show that the obtained representations can be utilized in an open-set recognition setting where no sample of the unknowns is available during training to capture the distinction between the knowns and the unknowns by exploiting the training samples of the knowns. We validate our approach on open-set identification and classification.

---

## 6. References

[1] D. Temel, J. Lee, and G. AlRegib, "Cure-or: Challenging unreal and real environments for object recognition," in *2018 17th IEEE International Conference on Machine Learning and Applications (ICMLA)*. IEEE, 2018, pp. 137–144.

[2] D. Temel, J. Lee, and G. AlRegib, "Object recognition under multifarious conditions: A reliability analysis and a feature similarity-based performance estimation," in *2019 IEEE International Conference on Image Processing (ICIP)*. IEEE, 2019, pp. 3033–3037.

[3] I. J. Goodfellow, J. Shlens, and C. Szegedy, "Explaining and harnessing adversarial examples," *arXiv preprint arXiv:1412.6572*, 2014.

[4] C. Guo, G. Pleiss, Y. Sun, and K. Q. Weinberger, "On calibration of modern neural networks," in *Proceedings of the 34th International Conference on Machine Learning*, vol. 70. JMLR.org, 2017, pp. 1321–1330.

[5] W. J. Scheirer, A. de Rezende Rocha, A. Sapkota, and T. E. Boult, "Toward open set recognition," *IEEE Transactions on Pattern Analysis and Machine Intelligence*, vol. 35, no. 7, pp. 1757–1772, 2012.

[6] A. Bendale and T. E. Boult, "Towards open set deep networks," in *Proceedings of the IEEE Conference on Computer Vision and Pattern Recognition*, 2016, pp. 1563–1572.

[7] R. Yoshihashi, W. Shao, R. Kawakami, S. You, M. Iida, and T. Naemura, "Classification-reconstruction learning for open-set recognition," in *Proceedings of the IEEE Conference on Computer Vision and Pattern Recognition*, 2019, pp. 4016–4025.

[8] P. Oza and V. M. Patel, "C2AE: Class conditioned auto-encoder for open-set recognition," in *Proceedings of the IEEE Conference on Computer Vision and Pattern Recognition*, 2019, pp. 2307–2316.

[9] X. Sun, Z. Yang, C. Zhang, K.-V. Ling, and G. Peng, "Conditional Gaussian distribution learning for open set recognition," in *Proceedings of the IEEE/CVF Conference on Computer Vision and Pattern Recognition*, 2020, pp. 13 480–13 489.

[10] Z. Ge, S. Demyanov, Z. Chen, and R. Garnavi, "Generative OpenMax for multi-class open set classification," in *Proceedings of the British Machine Vision Conference (BMVC)*, 2017, pp. 42.1–42.12.

[11] L. Neal, M. Olson, X. Fern, W.-K. Wong, and F. Li, "Open set learning with counterfactual images," in *Proceedings of the European Conference on Computer Vision (ECCV)*, 2018, pp. 613–628.

[12] J. Lee and G. AlRegib, "Gradients as a measure of uncertainty in neural networks," in *2020 IEEE International Conference on Image Processing (ICIP)*. IEEE, 2020, pp. 2416–2420.

[13] S. Ruder, "An overview of gradient descent optimization algorithms," *arXiv preprint arXiv:1609.04747*, 2016.

[14] M. D. Zeiler and R. Fergus, "Visualizing and understanding convolutional networks," in *European Conference on Computer Vision*. Springer, 2014, pp. 818–833.

[15] R. R. Selvaraju, M. Cogswell, A. Das, R. Vedantam, D. Parikh, and D. Batra, "Grad-CAM: Visual explanations from deep networks via gradient-based localization," in *Proceedings of the IEEE International Conference on Computer Vision*, 2017, pp. 618–626.

[16] M. Prabhushankar, G. Kwon, D. Temel, and G. AlRegib, "Contrastive explanations in neural networks," in *2020 IEEE International Conference on Image Processing (ICIP)*. IEEE, 2020, pp. 3289–3293.

[17] A. Madry, A. Makelov, L. Schmidt, D. Tsipras, and A. Vladu, "Towards deep learning models resistant to adversarial attacks," *arXiv preprint arXiv:1706.06083*, 2017.

[18] P. Oberdiek, M. Rottmann, and H. Gottschalk, "Classification uncertainty of deep neural networks based on gradient information," in *IAPR Workshop on Artificial Neural Networks in Pattern Recognition*. Springer, 2018, pp. 113–125.

[19] G. Kwon, M. Prabhushankar, D. Temel, and G. AlRegib, "Distorted representation space characterization through backpropagated gradients," in *2019 26th IEEE International Conference on Image Processing (ICIP)*, 2019.

[20] Y. Sun, M. Prabhushankar, and G. AlRegib, "Implicit saliency in deep neural networks," in *2020 IEEE International Conference on Image Processing (ICIP)*. IEEE, 2020, pp. 2915–2919.

[21] G. Kwon, M. Prabhushankar, D. Temel, and G. AlRegib, "Backpropagated gradient representations for anomaly detection," *arXiv preprint arXiv:2007.09507*, 2020.

[22] A. Krizhevsky, G. Hinton, et al., "Learning multiple layers of features from tiny images," 2009.

[23] S. Liang, Y. Li, and R. Srikant, "Enhancing the reliability of out-of-distribution image detection in neural networks," *arXiv preprint arXiv:1706.02690*, 2017.

---

*Published in: IEEE International Conference on Image Processing (ICIP), Anchorage, Alaska, USA, 2021.*  
*ISBN: 978-1-6654-4115-5/21/$31.00 ©2021 IEEE*
