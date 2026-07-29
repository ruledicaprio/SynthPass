# ICAO Doc 9303
## Machine Readable Travel Documents
### Part 10: Logical Data Structure (LDS) for Storage of Biometrics and Other Data in the Contactless Integrated Circuit (IC)
**Eighth Edition, 2021**

Approved by and published under the authority of the Secretary General
**INTERNATIONAL CIVIL AVIATION ORGANIZATION**

Published in separate English, Arabic, Chinese, French, Russian and Spanish editions by the
INTERNATIONAL CIVIL AVIATION ORGANIZATION
999 Robert-Bourassa Boulevard, Montréal, Quebec, Canada H3C 5H7
Downloads and additional information are available at [https://www.icao.int/publications/doc-series](https://www.icao.int/publications/doc-series)

**Doc 9303, Machine Readable Travel Documents**
Part 10 — Logical Data Structure (LDS) for Storage of Biometrics and Other Data in the Contactless Integrated Circuit (IC)  
Order No.: 9303P10  
ISBN 978-92-9265-418-4 (print version)  
ISBN 978-92-9275-992-6 (electronic version)  

© ICAO 2021  
All rights reserved. No part of this publication may be reproduced, stored in a retrieval system or transmitted in any form or by any means, without prior permission in writing from the International Civil Aviation Organization.

---

### AMENDMENTS AND CORRIGENDA

| No. | Date | Entered by |
| :--- | :--- | :--- |
| 1 | 14/6/24 | ICAO |
| 2 | 23/2/26 | ICAO |

*The designations employed and the presentation of the material in this publication do not imply the expression of any opinion whatsoever on the part of ICAO concerning the legal status of any country, territory, city or area or of its authorities, or concerning the delimitation of its frontiers or boundaries.*

---

## TABLE OF CONTENTS

1. [SCOPE](#1-scope)
2. [LOGICAL DATA STRUCTURE (LDS)](#2-logical-data-structure-lds)
   - 2.1 General
   - 2.2 eMRTD Application
   - 2.3 Elementary Files
   - 2.4 Data Groups
   - 2.5 Access Conditions
3. [LDS1 DATA GROUPS](#3-lds1-data-groups)
   - 3.1 EF.DG1 — Details Recorded in MRZ
   - 3.2 EF.DG2 — Encoded Identification Features (Face)
   - 3.3 EF.DG3 — Encoded Identification Features (Fingers)
   - 3.4 EF.DG4 — Encoded Identification Features (Iris)
   - 3.5 EF.DG5 — Displayed Identification Features (Portrait)
   - 3.6 EF.DG6 — Reserved for Future Use
   - 3.7 EF.DG7 — Displayed Signature or Usual Mark
   - 3.8 EF.DG8 — Data Feature(s)
   - 3.9 EF.DG9 — Encoded Identification Features (Structure)
   - 3.10 EF.DG10 — Encoded Identification Features (Substance)
   - 3.11 EF.DG11 — Additional Personal Details
   - 3.12 EF.DG12 — Additional Document Details
   - 3.13 EF.DG13 — Optional Details
   - 3.14 EF.DG14 — Security Options
   - 3.15 EF.DG15 — Active Authentication Public Key Info
   - 3.16 EF.DG16 — Person(s) to be Notified
4. [LDS2 APPLICATIONS](#4-lds2-applications)
   - 4.1 Travel Records Application
   - 4.2 Visa Records Application
   - 4.3 Additional Biometrics Application
   - 4.4 Certificate Application
5. [FILE STRUCTURES AND COMMANDS](#5-file-structures-and-commands)
   - 5.1 File Selection
   - 5.2 Record Operations
   - 5.3 APDU Commands
6. [DIGITAL SIGNATURES](#6-digital-signatures)
7. [REFERENCES (NORMATIVE)](#7-references-normative)
- **APPENDIX A** — LDS1 Data Group Summary
- **APPENDIX B** — LDS2 Data Group Summary
- **APPENDIX C** — Inspection Systems
- **APPENDIX D** — eMRTD Application File Structure
- **APPENDIX E** — File Structures Summary
- **APPENDIX F** — LDS Authorization Summary
- **APPENDIX G** — LDS Digital Signature Summary
- **APPENDIX H** — Example Reading Travel Records
- **APPENDIX I** — Example Searching Records by State
- **APPENDIX J** — Example Writing Travel Record and Certificate

---

## 1. SCOPE

This part of Doc 9303 specifies the Logical Data Structure (LDS) for storage of biometrics and other data in the contactless integrated circuit (IC) of machine readable travel documents (MRTDs).

The LDS defines:
- The file structure within the contactless IC;
- The data elements stored in each file;
- The encoding rules for biometric and other data;
- The access conditions for reading and writing data;
- The digital signature mechanisms for data integrity and authenticity.

Two versions of the LDS are defined:
- **LDS1** (REQUIRED): The baseline logical data structure for the eMRTD Application, containing biometric data and document details necessary for border control.
- **LDS2** (OPTIONAL): An extended logical data structure supporting additional applications including Travel Records, Visa Records, Additional Biometrics, and Certificates.

This part shall be read in conjunction with:
- Part 1 — *Introduction*;
- Part 11 — *Security Mechanisms for MRTDs*;
- Part 12 — *Public Key Infrastructure for MRTDs*.

---

## 2. LOGICAL DATA STRUCTURE (LDS)

### 2.1 General

The contactless IC of an eMRTD is organized as a file system conforming to [ISO/IEC 7816-4]. The logical structure consists of:

- A **Master File (MF)** at the root;
- One or more **Dedicated Files (DF)** representing applications;
- **Elementary Files (EF)** containing the actual data.

The eMRTD Application (LDS1) is the primary application and MUST be present. LDS2 applications are OPTIONAL extensions.

### 2.2 eMRTD Application

The eMRTD Application is identified by the Application Identifier (AID): `A0 00 00 02 47 10 01`.

It contains the following files:
- **EF.DIR** (`2F 00`): Application directory
- **EF.COM** (`60`): Common data (version, tag lists)
- **EF.SOD** (`77`): Security Object (digital signature)
- **EF.DG1** through **EF.DG16** (`61` through `70`): Data Groups

### 2.3 Elementary Files

#### 2.3.1 EF.DIR — Application Directory

EF.DIR contains the list of applications available on the IC. It is a transparent file.

| Tag | Description | Presence |
| :--- | :--- | :---: |
| `61` | Application template | M |
| `4F` | Application Identifier (AID) | M |
| `50` | Application label | O |
| `73` | Discretionary data objects | O |

#### 2.3.2 EF.COM — Common Data

EF.COM contains version information and lists of tags present in the LDS.

| Tag | Description | Presence |
| :--- | :--- | :---: |
| `5F01` | LDS version number | M |
| `5F36` | Unicode version number | M |
| `5C` | Tag list of data groups present | M |

#### 2.3.3 EF.SOD — Document Security Object

EF.SOD contains the digital signature over the hash values of all Data Groups. It is encoded as a PKCS#7/CMS SignedData structure.

| Component | Description | Presence |
| :--- | :--- | :---: |
| `77` | Security Object template | M |
| `06` | OID (id-signedData) | M |
| `A0` | Signed data | M |

### 2.4 Data Groups

Data Groups (DG1–DG16) store the actual biometric and document data. Each DG is an Elementary File with a specific structure (transparent or linear).

**Table 1. Data Group Summary (LDS1)**

| DG | Tag | File ID | Content | Structure | Access |
| :---: | :---: | :---: | :--- | :---: | :---: |
| 1 | `61` | `01 01` | Details recorded in MRZ | Transparent | Read: Always |
| 2 | `75` | `01 02` | Encoded Identification Features (Face) | Transparent | Read: BAC/PACE |
| 3 | `63` | `01 03` | Encoded Identification Features (Fingers) | Transparent | Read: BAC/PACE + TA |
| 4 | `76` | `01 04` | Encoded Identification Features (Iris) | Transparent | Read: BAC/PACE + TA |
| 5 | `65` | `01 05` | Displayed Identification Features (Portrait) | Transparent | Read: BAC/PACE |
| 6 | `66` | `01 06` | Reserved for Future Use | Transparent | — |
| 7 | `67` | `01 07` | Displayed Signature or Usual Mark | Transparent | Read: BAC/PACE |
| 8 | `68` | `01 08` | Data Feature(s) | Transparent | Read: BAC/PACE |
| 9 | `69` | `01 09` | Encoded Identification Features (Structure) | Transparent | Read: BAC/PACE + TA |
| 10 | `6A` | `01 0A` | Encoded Identification Features (Substance) | Transparent | Read: BAC/PACE + TA |
| 11 | `6B` | `01 0B` | Additional Personal Details | Transparent | Read: BAC/PACE |
| 12 | `6C` | `01 0C` | Additional Document Details | Transparent | Read: BAC/PACE |
| 13 | `6D` | `01 0D` | Optional Details | Transparent | Read: BAC/PACE |
| 14 | `6E` | `01 0E` | Security Options | Transparent | Read: BAC/PACE |
| 15 | `6F` | `01 0F` | Active Authentication Public Key Info | Transparent | Read: BAC/PACE |
| 16 | `70` | `01 10` | Person(s) to be Notified | Transparent | Read: BAC/PACE |

### 2.5 Access Conditions

Access to Data Groups is controlled by the security mechanisms defined in [Doc 9303-11](Doc_9303_Part11_Security_Mechanisms_for_MRTDs.md):
- **Always**: No authentication required (DG1 only).
- **BAC/PACE**: Requires successful Basic Access Control or PACE.
- **BAC/PACE + TA**: Requires BAC/PACE plus successful Terminal Authentication.

---

## 3. LDS1 DATA GROUPS

### 3.1 EF.DG1 — Details Recorded in MRZ

EF.DG1 contains the data recorded in the Machine Readable Zone. It is the only Data Group that can be read without any prior authentication.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `61` | DG1 template | M | Constructed |
| `5F1F` | MRZ data | M | B (variable) |

The MRZ data is encoded as a string of characters conforming to the MRZ specifications in [Doc 9303-3](Doc_9303_Part3_Specs_Common_to_all_MRTDs.md) (TD1), [Doc 9303-4](Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md) (TD2), or [Doc 9303-5](Doc_9303_Part5_Specs_for_TD1_MROTDs.md) (TD3).

### 3.2 EF.DG2 — Encoded Identification Features (Face)

EF.DG2 contains the facial image or facial template used for biometric verification.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `75` | DG2 template | M | Constructed |
| `5F2E` / `7F2E` | Biometric Information Template (BIT) | M | B |

The facial data MUST be encoded according to [ISO/IEC 19794-5] (face image data format).

### 3.3 EF.DG3 — Encoded Identification Features (Fingers)

EF.DG3 contains fingerprint images or fingerprint minutiae data.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `63` | DG3 template | M | Constructed |
| `5F2E` / `7F2E` | Biometric Information Template (BIT) | M | B |

Fingerprint data MUST be encoded according to [ISO/IEC 19794-4] (finger image data format) or [ISO/IEC 19794-2] (finger minutiae data format).

*Note.— Access to EF.DG3 requires Terminal Authentication in addition to BAC/PACE.*

### 3.4 EF.DG4 — Encoded Identification Features (Iris)

EF.DG4 contains iris image data.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `76` | DG4 template | M | Constructed |
| `5F2E` / `7F2E` | Biometric Information Template (BIT) | M | B |

Iris data MUST be encoded according to [ISO/IEC 19794-6] (iris image data format).

*Note.— Access to EF.DG4 requires Terminal Authentication in addition to BAC/PACE.*

### 3.5 EF.DG5 — Displayed Identification Features (Portrait)

EF.DG5 contains a displayed portrait image (not used for automated biometric matching).

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `65` | DG5 template | M | Constructed |
| `5F40` | Compressed image template | M | B |

The image format SHOULD be JPEG or JPEG2000.

### 3.6 EF.DG6 — Reserved for Future Use

EF.DG6 is reserved for future standardization. It SHALL NOT be used in the current edition.

### 3.7 EF.DG7 — Displayed Signature or Usual Mark

EF.DG7 contains an image of the holder's signature or usual mark.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `67` | DG7 template | M | Constructed |
| `5F43` | Signature or mark image template | M | B |

### 3.8 EF.DG8 — Data Feature(s)

EF.DG8 contains data features related to the document or holder.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `68` | DG8 template | M | Constructed |
| `5C` | Tag list | O | B |

### 3.9 EF.DG9 — Encoded Identification Features (Structure)

EF.DG9 contains structural biometric features.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `69` | DG9 template | M | Constructed |
| `5F2E` / `7F2E` | Biometric Information Template (BIT) | M | B |

*Note.— Access to EF.DG9 requires Terminal Authentication.*

### 3.10 EF.DG10 — Encoded Identification Features (Substance)

EF.DG10 contains substance-based biometric features.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `6A` | DG10 template | M | Constructed |
| `5F2E` / `7F2E` | Biometric Information Template (BIT) | M | B |

*Note.— Access to EF.DG10 requires Terminal Authentication.*

### 3.11 EF.DG11 — Additional Personal Details

EF.DG11 contains additional personal details of the document holder.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `6B` | DG11 template | M | Constructed |
| `5F0E` | Full name (native characters) | O | M |
| `5F11` | Other names | O | M |
| `5F42` | Personal number | O | M |
| `5F12` | Date of birth (full) | O | N (8) |
| `5F31` | Place of birth | O | M |
| `5F32` | Permanent address | O | M |
| `5F33` | Telephone number | O | M |
| `5F34` | Profession | O | M |
| `5F35` | Title | O | M |
| `5F36` | Personal summary | O | M |
| `5F57` | Date of birth (truncated) | O | N |
| `5F37` | Proof of citizenship | O | B |
| `5F38` | Other valid TD numbers | O | M |

### 3.12 EF.DG12 — Additional Document Details

EF.DG12 contains additional details about the travel document itself.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `6C` | DG12 template | M | Constructed |
| `5F55` | Date and time of personalization | O | N (14) |
| `5F56` | Serial number of personalization system | O | M |
| `5C` | Content of other fields | O | B |

### 3.13 EF.DG13 — Optional Details

EF.DG13 is reserved for optional data defined by the issuing State.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `6D` | DG13 template | M | Constructed |
| `5C` | Tag list of optional data | O | B |

### 3.14 EF.DG14 — Security Options

EF.DG14 contains the SecurityInfos structure indicating supported security protocols.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `6E` | DG14 template | M | Constructed |
| `7F6E` | SecurityInfos | M | Constructed |

The SecurityInfos structure is defined in [Doc 9303-11](Doc_9303_Part11_Security_Mechanisms_for_MRTDs.md), Section 9.2.

### 3.15 EF.DG15 — Active Authentication Public Key Info

EF.DG15 contains the public key used for Active Authentication.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `6F` | DG15 template | M | Constructed |
| `7F49` | Subject Public Key Info | M | Constructed |

### 3.16 EF.DG16 — Person(s) to be Notified

EF.DG16 contains information about person(s) to be notified in case of emergency.

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `70` | DG16 template | M | Constructed |
| `5F13` | Details of person(s) to be notified | O | M |

---

## 4. LDS2 APPLICATIONS

LDS2 extends the eMRTD with additional applications. These are OPTIONAL and stored as separate Dedicated Files (DFs) within the contactless IC.

### 4.1 Travel Records Application

The Travel Records Application stores records of border crossings.

**AID**: `A0 00 00 02 47 20 01`

**File Structure**:
- **EF.COM** (`60`): Common data
- **EF.SOD** (`77`): Security Object
- **EF.TravelRecords** (`01 01`): Linear file containing Travel Records

**Travel Record Structure**:

| Tag | Description | Presence | Format |
| :--- | :--- | :---: | :--- |
| `7F75` | Travel Record template | M | Constructed |
| `5F44` | Entry/Exit indicator | M | B (1) |
| `5F38` | Date of entry/exit | M | N (6 or 8) |
| `5F1F` | Port of entry/exit | O | M |
| `5F73` | Number of entries | O | V(1) |

*Note.— Within each EF supporting a linear structure, record numbers MUST be sequentially assigned when appending, such as in the order of creation; the first record (number one) is the first created record.*

### 4.2 Visa Records Application

The Visa Records Application stores visa records.

**AID**: `A0 00 00 02 47 20 02`

**File Structure**:
- **EF.COM** (`60`): Common data
- **EF.SOD** (`77`): Security Object
- **EF.VisaRecords** (`01 01`): Linear file containing Visa Records

### 4.3 Additional Biometrics Application

The Additional Biometrics Application stores secondary biometric templates.

**AID**: `A0 00 00 02 47 20 03`

**File Structure**:
- **EF.COM** (`60`): Common data
- **EF.SOD** (`77`): Security Object
- **EF.AdditionalBiometrics** (`01 01`): Transparent or linear file

### 4.4 Certificate Application

The Certificate Application stores X.509 certificates related to the document holder or document.

**AID**: `A0 00 00 02 47 20 04`

**File Structure**:
- **EF.COM** (`60`): Common data
- **EF.SOD** (`77`): Security Object
- **EF.Certificates** (`01 01`): Linear file containing certificates

---

## 5. FILE STRUCTURES AND COMMANDS

### 5.1 File Selection

Files are selected using the `SELECT` command as defined in [ISO/IEC 7816-4].

| Parameter | Value | Description |
| :--- | :--- | :--- |
| **CLA** | `0x00` | |
| **INS** | `0xA4` | SELECT |
| **P1** | `0x04` | Select by DF name |
| **P2** | `0x0C` | First record, no FCI |
| **Data** | AID | Application Identifier |

### 5.2 Record Operations

The following [ISO/IEC 7816-4] commands MUST be used for records access:
- **APPEND RECORD**: Appending of Travel Records, Visas, Certificates;
- **READ RECORD(S)**: Reading of one or more Travel Records, Visas, Certificates;
- **SEARCH RECORD**: Searching of one or more Travel Records, Visas, Certificates.

### 5.3 APDU Commands

#### 5.3.1 READ RECORD

| Parameter | Value | Description |
| :--- | :--- | :--- |
| **CLA** | `0x00` | |
| **INS** | `0xB2` | READ RECORD |
| **P1** | Record number | `0x00` for current |
| **P2** | `0x04` / `0x05` | Reference control |
| **Le** | `0x00` / expected | Expected length |

#### 5.3.2 APPEND RECORD

| Parameter | Value | Description |
| :--- | :--- | :--- |
| **CLA** | `0x00` | |
| **INS** | `0xE2` | APPEND RECORD |
| **P1** | `0x00` | |
| **P2** | `0x00` | |
| **Data** | Record data | TLV-encoded record |

#### 5.3.3 SEARCH RECORD

| Parameter | Value | Description |
| :--- | :--- | :--- |
| **CLA** | `0x00` | |
| **INS** | `0xA2` | SEARCH RECORD |
| **P1** | `0x00` | Search from beginning |
| **P2** | `0xF8` | Reference control |
| **Data** | Search template | TLV-encoded search criteria |

---

## 6. DIGITAL SIGNATURES

The Document Security Object (EF.SOD) contains a digital signature that covers the hash values of all Data Groups. The signature mechanism is defined in [Doc 9303-12](Doc_9303_Part12_Public_Key_Infrastructure_for_MRTDs.md) (PKI).

**Signature Process**:
1. Hash each Data Group (SHA-256 or SHA-512).
2. Construct the `SignedAttributes` including the hash values.
3. Sign the `SignedAttributes` with the Document Signer's private key.
4. Store the resulting CMS SignedData structure in EF.SOD.

**Verification Process** (Passive Authentication):
1. Read EF.SOD.
2. Extract the Document Signer Certificate.
3. Validate the certificate chain to a Trust Anchor.
4. Verify the signature over `SignedAttributes`.
5. Compare hash values with actual Data Group contents.

---

## 7. REFERENCES (NORMATIVE)

* **[ISO/IEC 7816-4]** Identification cards — Integrated circuit cards — Part 4: Organization, security and commands for interchange
* **[ISO/IEC 19794-2]** Information technology — Biometric data interchange formats — Part 2: Finger minutiae data
* **[ISO/IEC 19794-4]** Information technology — Biometric data interchange formats — Part 4: Finger image data
* **[ISO/IEC 19794-5]** Information technology — Biometric data interchange formats — Part 5: Face image data
* **[ISO/IEC 19794-6]** Information technology — Biometric data interchange formats — Part 6: Iris image data
* **[RFC 3369]** Cryptographic Message Syntax (CMS)
* **[RFC 5280]** Internet X.509 Public Key Infrastructure Certificate and CRL Profile

---

## APPENDICES (INFORMATIVE)

### APPENDIX A — LDS1 DATA GROUP SUMMARY

Summarizes all 16 Data Groups, their tags, file IDs, content descriptions, and access conditions as presented in Section 2.4.

### APPENDIX B — LDS2 DATA GROUP SUMMARY

Summarizes the LDS2 application structures, AIDs, and data elements for Travel Records, Visa Records, Additional Biometrics, and Certificates.

### APPENDIX C — INSPECTION SYSTEMS

#### C.1 Operating Volume and Test Positions
Defines the operating volume for contactless IC communication and standard test positions for conformance testing.

#### C.2 Particular Waveform and RF Requirements
Specifies RF carrier frequency (13.56 MHz), modulation, and bit rates for [ISO/IEC 14443] communication.

#### C.3 Polling Sequences and eMRTD Detection Time
Defines the polling sequence (Type A and Type B) and maximum detection time for eMRTDs.

#### C.4 Mandatory Bit Rates
106 kbit/s is MANDATORY. 212 kbit/s, 424 kbit/s, and 848 kbit/s are OPTIONAL.

#### C.5 Electromagnetic Disturbance (EMD)
Defines immunity requirements for eMRTDs operating in the presence of electromagnetic disturbances.

#### C.6 Supported Antenna Classes
Defines antenna size classes for eMRTDs (ID-1, ID-2, ID-3 formats).

#### C.7 (Optional) Frame Sizes and Error Correction
Defines optional support for larger frame sizes and forward error correction.

#### C.8 (Optional) Support of Additional Classes
Defines optional support for additional antenna and operating classes.

#### C.9 (Recommended) Operating Distance
RECOMMENDED operating distance of ≥ 4 cm for ID-3 documents.

### APPENDIX D — eMRTD APPLICATION FILE STRUCTURE

Provides a complete tree view of the eMRTD Application file hierarchy:

```text
MF (3F 00)
└── DF.eMRTD (A0 00 00 02 47 10 01)
    ├── EF.DIR (2F 00)
    ├── EF.COM (60)
    ├── EF.SOD (77)
    ├── EF.DG1 (01 01) — MRZ Details
    ├── EF.DG2 (01 02) — Face
    ├── EF.DG3 (01 03) — Fingers
    ├── EF.DG4 (01 04) — Iris
    ├── EF.DG5 (01 05) — Portrait
    ├── EF.DG6 (01 06) — Reserved
    ├── EF.DG7 (01 07) — Signature/Mark
    ├── EF.DG8 (01 08) — Data Features
    ├── EF.DG9 (01 09) — Structure
    ├── EF.DG10 (01 0A) — Substance
    ├── EF.DG11 (01 0B) — Personal Details
    ├── EF.DG12 (01 0C) — Document Details
    ├── EF.DG13 (01 0D) — Optional Details
    ├── EF.DG14 (01 0E) — Security Options
    ├── EF.DG15 (01 0F) — AA Public Key
    └── EF.DG16 (01 10) — Persons to Notify
```

### APPENDIX E — FILE STRUCTURES SUMMARY

Consolidated summary of all file structures across LDS1 and LDS2 applications, including file types (transparent, linear fixed, linear variable), record sizes, and maximum number of records.

### APPENDIX F — LDS AUTHORIZATION SUMMARY

**Table F-1. Authorization Matrix**

| Data / Operation | No Auth | BAC/PACE | BAC/PACE + TA | TA + SM |
| :--- | :---: | :---: | :---: | :---: |
| Read DG1 | ✓ | ✓ | ✓ | ✓ |
| Read DG2, DG5, DG7, DG11–DG16 | | ✓ | ✓ | ✓ |
| Read DG3, DG4, DG9, DG10 | | | ✓ | ✓ |
| Read DG14, DG15 | | ✓ | ✓ | ✓ |
| Read LDS2 Travel Records | | | ✓ | ✓ |
| Write LDS2 Travel Records | | | | ✓ |
| Read LDS2 Certificates | | | ✓ | ✓ |
| Write LDS2 Certificates | | | | ✓ |

### APPENDIX G — LDS DIGITAL SIGNATURE SUMMARY

Summarizes the digital signature requirements:
- EF.SOD MUST be present and contain a valid CMS SignedData structure.
- All Data Groups present in the LDS MUST be covered by the signature.
- The hash algorithm MUST be SHA-256 or SHA-512.
- The signature algorithm MUST be RSA (PKCS#1 v1.5 or PSS) or ECDSA.

### APPENDIX H — EXAMPLE READING TRAVEL RECORDS

#### H.1 FMM Command Retrieving the Number of Entry Records

```text
Command:  00 B2 00 04 00
Response: [Record data] 90 00
```

#### H.2 READ RECORD Command Retrieving the Last Travel Record from the Retrieved List

```text
Command:  00 B2 [Last] 04 00
Response: 7F75 [Len] [Travel Record Data] 90 00
```

#### H.3 READ RECORD Command Retrieving the Last Two Travel Records

```text
Command:  00 B2 [Last-1] 04 00
Response: 7F75 [Len] [Record N-1 Data] 90 00
Command:  00 B2 [Last] 04 00
Response: 7F75 [Len] [Record N Data] 90 00
```

### APPENDIX I — EXAMPLE SEARCHING RECORDS BY STATE

#### I.1 SEARCH RECORD Command Searching Travel Record(s) by Destination State

```text
Command:
  CLA: 00
  INS: A2
  P1:  00
  P2:  F8
  Lc:  Var
  Data: 7F76 [Len]
        51 01 01
        A1 0B
          80 01 00
          B0 06 02 01 03
          02 01 03
        A3 07
          B1 05
            81 03 [xx xx xx]  ← Destination State code
  Le:  00

Response:
  [List of matching record numbers] 90 00
```

### APPENDIX J — EXAMPLE WRITING TRAVEL RECORD AND CERTIFICATE

#### J.1 SEARCH RECORD Command Searching EF.Certificates by Certificate Serial Number

```text
Command:
  CLA: 00
  INS: A2
  P1:  00
  P2:  F8
  Data: [Search template with serial number]
  Le:  00
```

#### J.2 APPEND RECORD Command Writing Certificate

```text
Command:
  CLA: 00
  INS: E2
  P1:  00
  P2:  00
  Data: [Certificate TLV data]
```

#### J.3 APPEND RECORD Command Writing Travel Record

```text
Command:
  CLA: 00
  INS: E2
  P1:  00
  P2:  00
  Data: 7F75 [Len]
        5F44 01 [Entry/Exit indicator]
        5F38 [Len] [Date]
        [Optional fields...]
```

---
*— END —*
