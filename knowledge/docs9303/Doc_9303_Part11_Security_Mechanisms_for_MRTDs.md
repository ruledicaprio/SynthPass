# ICAO Doc 9303
## Machine Readable Travel Documents
### Part 11: Security Mechanisms for MRTDs
**Eighth Edition, 2021**

Approved by and published under the authority of the Secretary General  
**INTERNATIONAL CIVIL AVIATION ORGANIZATION**  

Published in separate English, Arabic, Chinese, French, Russian and Spanish editions by the  
INTERNATIONAL CIVIL AVIATION ORGANIZATION  
999 Robert-Bourassa Boulevard, Montréal, Quebec, Canada H3C 5H7  
Downloads and additional information are available at [https://www.icao.int/publications/doc-series](https://www.icao.int/publications/doc-series)

**Doc 9303, Machine Readable Travel Documents**  
Part 11 — Security Mechanisms for MRTDs  
Order No.: 9303P11  
ISBN 978-92-9265-419-1 (print version)  
ISBN 978-92-9275-993-3 (electronic version)  

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
2. [ASSUMPTIONS AND NOTATIONS](#2-assumptions-and-notations)
   - 2.1 Requirements for eMRTD Chips and Terminals
   - 2.2 Notations
3. [SECURING ELECTRONIC DATA](#3-securing-electronic-data)
4. [ACCESS TO THE CONTACTLESS IC](#4-access-to-the-contactless-ic)
   - 4.1 Compliant Configurations
   - 4.2 Chip Access Procedure
   - 4.3 Basic Access Control
   - 4.4 Password Authenticated Connection Establishment
5. [AUTHENTICATION OF DATA](#5-authentication-of-data)
   - 5.1 Passive Authentication
6. [AUTHENTICATION OF THE CONTACTLESS IC](#6-authentication-of-the-contactless-ic)
   - 6.1 Active Authentication
   - 6.2 Chip Authentication
7. [ADDITIONAL ACCESS CONTROL MECHANISMS](#7-additional-access-control-mechanisms)
   - 7.1 Terminal Authentication
   - 7.2 Encryption of Additional Biometrics
8. [INSPECTION SYSTEM](#8-inspection-system)
9. [COMMON SPECIFICATIONS](#9-common-specifications)
   - 9.1 ASN.1 Structures
   - 9.2 Information on Supported Protocols and Supported Applications
   - 9.3 APDUs
   - 9.4 Public Key Data Objects
   - 9.5 Domain Parameters
   - 9.6 Key Agreement Algorithms
   - 9.7 Key Derivation Mechanism
   - 9.8 Secure Messaging
10. [REFERENCES (NORMATIVE)](#10-references-normative)
- **APPENDIX A** — Entropy of MRZ-Derived Access Keys
- **APPENDIX B** — Point Encoding for the ECDH-Integrated Mapping
- **APPENDIX C** — Challenge Semantics
- **APPENDIX D** — Worked Example: Basic Access Control
- **APPENDIX E** — Worked Example: Passive Authentication
- **APPENDIX F** — Worked Example: Active Authentication
- **APPENDIX G** — Worked Example: PACE – Generic Mapping
- **APPENDIX H** — Worked Example: PACE – Integrated Mapping
- **APPENDIX I** — Worked Example: PACE – PACE CA Mapping
- **APPENDIX J** — Inspection Procedures
- **APPENDIX K** — European Extended Access Control

---

## 1. SCOPE

Part 11 to Doc 9303 provides specifications to enable States and suppliers to implement cryptographic security features for electronic machine readable travel documents (“eMRTDs”) offering contactless integrated circuit (IC) access.

Cryptographic protocols are specified to:
* prevent skimming of data from the contactless IC;
* prevent eavesdropping on the communication between contactless IC and reader;
* provide authentication of the data stored on the contactless IC based on the Public Key Infrastructure (PKI) described in Part 12; and
* provide authentication of the contactless IC itself.

The Eighth Edition of Doc 9303 incorporates the specifications for the optional Travel Records, Visa Records, and Additional Biometrics applications (known as LDS2 applications) as an optional extension of the eMRTD. This part of Doc 9303 includes the necessary extended access control protocols to protect writing and reading of the data of the respective LDS2 applications. These access control protocols may also be used for the protection of the secondary biometrics in the eMRTD Application.

* The authentication of the data stored on the contactless IC is the basic security feature to enable the use of the IC for manual and/or automated inspection. This feature is therefore **REQUIRED**.
* Implementation of a protocol to prevent skimming of the data stored on the contactless IC and to prevent eavesdropping on the communication between IC and terminal is **REQUIRED**.
* Implementation of the other protocols is **OPTIONAL**, allowing the issuing State or organization to decide on the necessary set of security features according to national regulations/demands.

This part shall be read in conjunction with the following Parts of Doc 9303:
* Part 1 — *Introduction*;
* Part 10 — *Logical Data Structure (LDS) for Storage of Biometrics and Other Data in the Contactless Integrated Circuit (IC)*; and
* Part 12 — *Public Key Infrastructure for MRTDs*.

---

## 2. ASSUMPTIONS AND NOTATIONS

It is assumed that the reader of this document is familiar with the concepts and mechanisms offered by public key cryptography and public key infrastructures.

While the use of public key cryptography techniques adds some complexity to the implementation of eMRTDs, such techniques add value in that they will provide front-line border control points with an additional measure to determine the authenticity of the eMRTD. It is assumed that the use of such a technique is not the sole measure for determining authenticity and it SHOULD NOT be relied upon as a single determining factor.

In the event that the data from the contactless IC cannot be used, for instance as a result of a certificate revocation or an invalid signature verification, or if the contactless IC was left intentionally blank (see Section 4.5.4 of Doc 9303-10), the eMRTD is not necessarily invalidated. In such cases a receiving State MAY rely on other document security features for validation purposes.

### 2.1 Requirements for eMRTD Chips and Terminals
This part of Doc 9303 specifies requirements for implementations of eMRTD chips (or, equivalently, IC) and terminals (or inspection systems). While eMRTD chips must comply with those requirements according to the terminology described in Doc 9303-1, requirements for terminals are to be interpreted as guidance, i.e. interoperability of eMRTD chip and terminal are only guaranteed if the terminal complies with those requirements, otherwise the interaction with the eMRTD chip will either fail or the behaviour of the eMRTD chip is undefined. In general, the eMRTD chip need not enforce requirements related to terminals unless the security of the eMRTD chip is directly affected.

### 2.2 Notations
The following notations are used to denote cryptographic primitives in an algorithm independent way:
* Encryption of clear text S with symmetric key K: **E**(K, S);
* Decryption of cipher text C with symmetric key K: **D**(K, C);
* The operation for computing a hash over a message m is denoted by **H**(m).
* Computing a Message Authentication Code with symmetric key K over message M: **MAC**(K,M);
* Key agreement based on asymmetric key pairs (SK, PK) and (SK’, PK’) and domain parameters D: **KA**(SK,PK’,D) / **KA**(SK’,PK,D);
* Key derivation from a shared secret S: **KDF**(S);
* Signing a message m with private key SK_IFD is denoted by s = **Sign**(SK_IFD, m);
* Verifying the resulting signature s with public key PK_IFD and message m: **Verify**(PK_IFD, s, m).
* Computing a compressed representation of a public key PK: **Comp**(PK).

---

## 3. SECURING ELECTRONIC DATA

Besides Passive Authentication by digital signatures and Chip Access Control, issuing States or organizations MAY choose additional security, using more complex ways of securing the contactless IC and its data.

Accessing an eMRTD comprises the following steps:
1. Gain access to the contactless IC of the eMRTD (Section 4)
2. Authentication of data (Section 5)
3. Authentication of the chip (Section 6)
4. Additional access control mechanisms (Section 7)
5. Reading data (see Doc 9303-10).

Different protocols are available for the different steps. The exact configuration of an eMRTD is chosen by the issuing State or organization. The options given in Table 1 can be suitably combined to achieve additional security according to the requirements of issuers. Inspection Procedures for different configurations of eMRTDs are described in Appendix J.

**Table 1. Securing Electronic Data (Summary)**

| Method | Contactless IC | Inspection System | Benefits | Note |
| :--- | :---: | :---: | :--- | :--- |
| **BASELINE SECURITY METHOD** | | | | |
| Passive Authentication (5.1) | m | m | Proves that the contents of the SOD and the LDS are authentic and not changed. | Does not prevent an exact copy or IC substitution. Does not prevent unauthorized access/skimming. |
| **ADVANCED SECURITY METHODS** | | | | |
| Comparison of conventional MRZ and IC-based MRZ | n/a | o | Proves that contactless IC’s content and physical eMRTD belong together. | Adds (minor) complexity. Does not prevent an exact copy. |
| Active Authentication (6.1) | o | o | Prevents copying the SOD. Proves IC has not been substituted. | Does not prevent unauthorized access. Adds complexity. Chip Auth is REQUIRED for LDS2. |
| Chip Authentication (6.2) | o/c | o | Prevents copying the SOD. Proves IC has not been substituted. | Does not prevent unauthorized access. Adds complexity. Chip Auth is REQUIRED for LDS2. |
| Basic Access Control (BAC) (4.3) | c | m | Prevents skimming/misuse. Prevents eavesdropping. | Does not prevent exact copy/IC substitution. At least BAC or PACE SHALL be supported. PACE is REQUIRED for LDS2. |
| Password Authenticated Connection Establishment (PACE) (4.4) | r/c | m | Prevents skimming/misuse. Prevents eavesdropping. | Does not prevent exact copy/IC substitution. At least BAC or PACE SHALL be supported. PACE is REQUIRED for LDS2. PACE offers better protection than BAC. |
| Terminal Authentication (7.1) | o/c | o | Prevents unauthorized access/skimming of sensitive data. | Requires additional key management. Terminal Auth is REQUIRED for LDS2. |
| Data Encryption (7.2) | o | o | Secures additional biometrics. Does not require processor-ICs. | Requires complex decryption key management. |

*m = REQUIRED, r = RECOMMENDED, o = OPTIONAL, c = CONDITIONAL, n/a = not applicable.*

---

## 4. ACCESS TO THE CONTACTLESS IC

Adding a contactless IC without access control to an eMRTD introduces two new attack possibilities:
* the data stored in the contactless IC can be electronically read without authorizing this reading of the document (skimming); and
* the unencrypted communication between a contactless IC and a reader can be eavesdropped within a distance of several metres.

Therefore, it is understood that issuing States or organizations SHALL implement a Chip Access Control mechanism. This section defines two mechanisms for Chip Access Control:
* **Basic Access Control (BAC, Section 4.3)**, which is based purely on symmetric cryptography; and
* **Password Authenticated Connection Establishment (PACE, Section 4.4)**, which employs asymmetric cryptography to provide higher entropy session keys.

### 4.1 Compliant Configurations
The following configurations comply with this specification:
* eMRTD chips implementing BAC only;
* eMRTD chips implementing PACE *and* BAC;
* eMRTD chips implementing PACE only.

The security provided by Basic Access Control is limited by the design of the protocol. Therefore, a gradual change over from BAC to PACE is agreed:
* Issuing States SHALL implement PACE as of **1 January 2027**.
* Issuing States SHALL NOT issue eMRTDs with BAC as of **1 January 2028** (PACE only).
* Issuing States SHALL ensure that all eMRTDs with BAC are out of circulation by **1 January 2038**.

*Note 1.— Previous versions of Doc 9303 allowed eMRTD chips implementing no Chip Access Control (“plain eMRTDs”). This is deprecated in the Eighth Edition.*  
*Note 2.— For access to LDS2 applications, the IC MUST require the execution of PACE.*

### 4.2 Chip Access Procedure
The chip access procedure to authenticate the inspection system consists of the following steps:
1. **Read EF.CardAccess (REQUIRED)**: If PACE is supported, the inspection system SHALL read EF.CardAccess to determine supported parameters.
2. **Read EF.DIR (OPTIONAL)**: The Inspection System MAY read EF.DIR to retrieve a list of applications.
3. **PACE (CONDITIONAL)**: RECOMMENDED if supported. REQUIRED for LDS2. The inspection system derives $K_\pi$ from MRZ or CAN. Mutual authentication is performed, and Secure Messaging is started.
4. **Basic Access Control (CONDITIONAL)**: REQUIRED if Chip Access Control is enforced and PACE has not been used.

### 4.3 Basic Access Control

#### 4.3.1 Protocol Specification
Authentication and Key Establishment is provided by a three-pass challenge-response protocol using 3DES as block cipher. A cryptographic checksum according to [ISO/IEC 9797-1] MAC Algorithm 3 is calculated over and appended to the ciphertexts.

1. The IFD requests a challenge `RND.IC` by sending the `GET CHALLENGE` command.
2. The IFD generates `RND.IFD` and `K.IFD`, computes cryptogram $E_{IFD}$ and checksum $M_{IFD}$, and sends `EXTERNAL AUTHENTICATE`.
3. The IC verifies, decrypts, generates `K.IC`, computes $E_{IC}$ and $M_{IC}$, and responds.
4. The IFD verifies and decrypts.
5. Both derive session keys $K_{SEnc}$ and $K_{SMAC}$ using `(K.IC xor K.IFD)` as shared secret.

#### 4.3.2 Inspection Process
The inspection system reads the “MRZ_information” (Document Number, Date of Birth, Date of Expiry + check digits). The most significant 16 bytes of the SHA-1 hash of this string are used as key seed to derive the Document Basic Access Keys ($K_{Enc}$ and $K_{MAC}$).

#### 4.3.3 Cryptographic Specifications
* **Encryption**: Two-key 3DES in CBC mode with zero IV.
* **Authentication**: [ISO/IEC 9797-1] MAC algorithm 3 with block cipher DES, zero IV, padding method 2. MAC length MUST be 8 bytes.

#### 4.3.4 Application Protocol Data Units

**GET CHALLENGE**
| Parameter | Value | Description |
| :--- | :--- | :--- |
| **CLA** | Context specific | |
| **INS** | `0x84` | GET CHALLENGE |
| **P1/P2** | `0x0000` | — |
| **Data** | | Absent |
| **Response Data** | Random Nonce | |
| **SW** | `0x9000` | Normal processing |
| | `Other` | Error |

**EXTERNAL AUTHENTICATE**
| Parameter | Value | Description |
| :--- | :--- | :--- |
| **CLA** | Context specific | |
| **INS** | `0x82` | EXTERNAL AUTHENTICATE |
| **P1/P2** | `0x0000` | — |
| **Data** | $E_{IFD} \parallel M_{IFD}$ | REQUIRED |
| **Response Data** | $E_{IC} \parallel M_{IC}$ | REQUIRED |
| **SW** | `0x9000` | Normal processing |
| | `Other` | Error |

### 4.4 Password Authenticated Connection Establishment (PACE)
PACE is a password authenticated Diffie-Hellman key agreement protocol. It establishes Secure Messaging based on weak (short) passwords (e.g., MRZ or CAN).
* Strong session keys are provided independent of the strength of the password.
* PACE supports different Mappings: **Generic Mapping**, **Integrated Mapping**, and **Chip Authentication Mapping**.

#### 4.4.1 Protocol Specification
1. The IC randomly chooses a nonce $s$, encrypts it to $z = E(K_\pi, s)$, and sends $z$.
2. The terminal recovers $s$.
3. Both map the nonce to ephemeral domain parameters $D = Map(D_{IC}, s, ...)$.
4. Both perform anonymous Diffie-Hellman key agreement to generate shared secret $K$.
5. Both derive session keys $K_{SMAC}$ and $K_{SEnc}$.
6. Both exchange and verify authentication tokens $T_{IFD}$ and $T_{IC}$.
7. Conditionally, the IC computes and sends Encrypted Chip Authentication Data.

#### 4.4.3 Cryptographic Specifications
*(Tables of OIDs for DH and ECDH mappings omitted for brevity, see Section 9.2.3 for full OID trees).*
* **3DES**: Retail-mode MAC algorithm 3.
* **AES**: CMAC-mode with MAC length of 8 bytes.

#### 4.4.4 Application Protocol Data Units
PACE is implemented using a chain of `GENERAL AUTHENTICATE` commands.
1. `MSE:Set AT` (Select and initialize PACE)
2. `GENERAL AUTHENTICATE` (Execute protocol steps)

---

## 5. AUTHENTICATION OF DATA

### 5.1 Passive Authentication
Passive Authentication proves that the contents of the Document Security Object (SOD) and LDS are authentic and not changed. It does not prevent exact copying or chip substitution.

**Inspection Process:**
1. Read the SOD (containing the Document Signer Certificate).
2. Build and validate a certification path from a Trust Anchor to the Document Signer Certificate.
3. Verify the signature of the SOD.
4. Read relevant Data Groups.
5. Hash the contents and compare with the corresponding hash values in the SOD.

---

## 6. AUTHENTICATION OF THE CONTACTLESS IC

### 6.1 Active Authentication
Active Authentication authenticates the IC by signing a challenge sent by the IFD with a private key known only to the IC. The public key is stored in Data Group 15 and protected by the SOD.
* Performed using the `INTERNAL AUTHENTICATE` command.
* Supports RSA (ISO/IEC 9796-2) and ECDSA.

### 6.2 Chip Authentication
The Chip Authentication Protocol is an ephemeral-static Diffie-Hellman key agreement protocol.
* Prevents Challenge Semantics (transcripts are non-transferable).
* Provides strong session keys.
* Performed using `MSE:Set KAT` (for 3DES) or `MSE:Set AT` + `GENERAL AUTHENTICATE` (for AES).

---

## 7. ADDITIONAL ACCESS CONTROL MECHANISMS

### 7.1 Terminal Authentication
Terminal Authentication is a two-move challenge-response protocol that provides explicit unilateral authentication of the terminal. It is REQUIRED for LDS2 applications.
* The terminal sends a certificate chain (CVCA -> DV -> Terminal).
* The IC verifies certificates, extracts $PK_{IFD}$, and sends a challenge $r_{IC}$.
* The terminal responds with signature $s_{IFD} = Sign(SK_{IFD}, ID_{IC} \parallel r_{IC} \parallel Comp(PK_{DH,IFD}))$.
* Access rights are bound to the Secure Messaging established by the authenticated ephemeral public key.

#### 7.1.4 Cryptographic Specifications
* **RSA**: RSASSA-PSS (SHA-256, SHA-512).
* **ECDSA**: Plain signature format (SHA-224, SHA-256, SHA-384, SHA-512).

#### 7.1.5 Application Protocol Data Units
Sequence of commands:
1. `MSE:Set DST`
2. `PSO:Verify Certificate`
3. `MSE:Set AT`
4. `Get Challenge`
5. `External Authenticate`

### 7.2 Encryption of Additional Biometrics
Restricting access to additional biometrics MAY be done by encrypting them. The inspection system MUST be provided with a decryption key.

---

## 8. INSPECTION SYSTEM

Inspection systems MUST meet pre-conditions to support the required functionality:
* **BAC/PACE**: Equipped with MRZ/CAN reading means and software supporting the protocols.
* **Passive Auth**: Securely stores Country Signing CA Certificates or Document Signer Certificates and has access to revocation information.
* **Active/Chip/Terminal Auth**: OPTIONAL support, but if supported, MUST implement the respective protocols and key management.

---

## 9. COMMON SPECIFICATIONS

### 9.1 ASN.1 Structures
```asn1
SubjectPublicKeyInfo ::= SEQUENCE {
    algorithm AlgorithmIdentifier,
    subjectPublicKey BIT STRING
}

AlgorithmIdentifier ::= SEQUENCE {
    algorithm OBJECT IDENTIFIER,
    parameters ANY DEFINED BY algorithm OPTIONAL
}
```

### 9.2 Information on Supported Protocols and Supported Applications
The ASN.1 data structure `SecurityInfos` indicates supported security protocols.

```asn1
SecurityInfos ::= SET OF SecurityInfo
SecurityInfo ::= SEQUENCE {
    protocol OBJECT IDENTIFIER,
    requiredData ANY DEFINED BY protocol,
    optionalData ANY DEFINED BY protocol OPTIONAL
}
```

#### 9.2.1 PACEInfo
```asn1
PACEInfo ::= SEQUENCE {
    protocol OBJECT IDENTIFIER (
        id-PACE-DH-GM-3DES-CBC-CBC | id-PACE-DH-GM-AES-CBC-CMAC-128 | ...
    ),
    version INTEGER, -- MUST be 2
    parameterId INTEGER OPTIONAL
}
```

#### 9.2.5 ChipAuthenticationInfo
```asn1
ChipAuthenticationInfo ::= SEQUENCE {
    protocol OBJECT IDENTIFIER (
        id-CA-DH-3DES-CBC-CBC | id-CA-DH-AES-CBC-CMAC-128 | ...
    ),
    version INTEGER, -- MUST be 1
    keyId INTEGER OPTIONAL
}
```

#### 9.2.8 TerminalAuthenticationInfo
```asn1
TerminalAuthenticationInfo ::= SEQUENCE {
    protocol OBJECT IDENTIFIER(id-TA),
    version INTEGER -- MUST be 1
}
```

### 9.3 APDUs
* **Extended Length**: REQUIRED for terminals. CONDITIONAL for eMRTD chips.
* **Command Chaining**: MUST be used for `GENERAL AUTHENTICATE` to link the sequence of commands to the execution of the PACE protocol.

### 9.4 Public Key Data Objects
Public keys are encoded as constructed BER TLV structures nested within the cardholder public key template `0x7F49`.
* **RSA**: Tags `0x06` (OID), `0x81` (Modulus), `0x82` (Exponent).
* **DH**: Tags `0x06` (OID), `0x81` (Prime), `0x82` (Order), `0x83` (Generator), `0x84` (Public Value).
* **ECDH**: Tags `0x06` (OID), `0x81` (Prime), `0x82` (a), `0x83` (b), `0x84` (Base point), `0x85` (Order), `0x86` (Public point), `0x87` (Cofactor).

### 9.5 Domain Parameters
Standardized domain parameters SHOULD be used. Explicit domain parameters MUST NOT use IDs reserved for standardized parameters.

**Table 12. Standardized domain parameters (Excerpt)**
| ID | Name | Size (bit) | Type | Reference |
| :---: | :--- | :--- | :---: | :--- |
| 0 | 1024-bit MODP Group | 1024/160 | GFP | [RFC 5114] |
| 8 | NIST P-192 (secp192r1) | 192 | ECP | [RFC 5114] |
| 12 | NIST P-256 (secp256r1) | 256 | ECP | [RFC 5114] |
| 13 | BrainpoolP256r1 | 256 | ECP | [RFC 5639] |
| 18 | NIST P-521 (secp521r1) | 521 | ECP | [RFC 5114] |

### 9.6 Key Agreement Algorithms
This specification supports Diffie-Hellman and Elliptic Curve Diffie-Hellman key agreement, summarized in Table 13.

**Table 13. Key Agreement Algorithms**
| Algorithm / Format | DH | ECDH |
| :--- | :--- | :--- |
| Key Agreement Algorithm | [PKCS#3] | ECKA [TR-03111] |
| X.509 Public Key Format | [PKCS#3] | [TR-03111] |
| TLV Public Key Format | TLV (see Section 9.4) | [TR-03111] |
| Ephemeral Public Key Validation | [RFC 2631] | TLV (see Section 9.4) / [TR-03111] |

### 9.7 Key Derivation Mechanism
```text
KDF(K, c):
  keydata = H(K || c)
  Output keydata
```
* **3DES**: SHA-1 is used. Octets 1-8 form keydataA, 9-16 form keydataB.
* **AES**: SHA-1 (for 128-bit) or SHA-256 (for 192/256-bit) is used.

### 9.8 Secure Messaging
Secure Messaging is based on either 3DES or AES in encrypt-then-authenticate mode.
* **Send Sequence Counter (SSC)**: 64-bit for 3DES, 128-bit for AES. Incremented before every command/response.
* **Message Structure**:
  * Command: `[DO‘85’ or DO‘87’] [DO‘97’] DO‘8E’`
  * Response: `[DO‘85’ or DO‘87’] [DO‘99’] DO‘8E’`
* **3DES Modes**: CBC encryption (zero IV), MAC Algorithm 3.
* **AES Modes**: CBC encryption (IV = E(KSEnc, SSC)), CMAC authentication.

---

## 10. REFERENCES (NORMATIVE)

* **[ISO/IEC 7816-4]** Identification cards — Integrated circuit cards — Part 4: Organization, security and commands for interchange
* **[ISO/IEC 9796-2]** Digital signature schemes giving message recovery — Part 2: Integer factorization based mechanisms
* **[ISO/IEC 9797-1]** Message Authentication Codes (MACs) — Part 1: Mechanisms using a block cipher
* **[FIPS 197]** Specification for the Advanced Encryption Standard (AES)
* **[PKCS#3]** RSA Laboratories, PKCS#3: Diffie-Hellman Key-agreement Standard
* **[RFC 2631]** Diffie-Hellman Key Agreement Method
* **[RFC 5114]** Additional Diffie-Hellman Groups for Use with IETF Standards
* **[RFC 5280]** Internet X.509 Public Key Infrastructure Certificate and CRL Profile
* **[RFC 5639]** Elliptic Curve Cryptography (ECC) Brainpool Standard Curves and Curve Generation
* **[TR-03110]** BSI: Technical Guideline TR-03110: Advanced Security Mechanisms for Machine Readable Travel Documents
* **[TR-03111]** BSI: Technical Guideline TR-03111: Elliptic Curve Cryptography

---

## APPENDICES (INFORMATIVE)

### APPENDIX A — ENTROPY OF MRZ-DERIVED ACCESS KEYS
Basic Access Control keys are generated from printed data with limited randomness. Maximum strength is approx. 56-bit (numeric) or 73-bit (alphanumeric). PACE was designed to overcome this by employing asymmetric cryptography to establish session keys whose strength is independent of the password's entropy.

### APPENDIX B — POINT ENCODING FOR THE ECDH-INTEGRATED MAPPING
Describes the algorithm for mapping octet strings to elements of GF(p) and subsequently to the cryptographic group for Integrated Mapping. Requires $p \equiv 3 \pmod 4$.

### APPENDIX C — CHALLENGE SEMANTICS
Discusses the risks of "Challenge Semantics" in Active Authentication, where a terminal could potentially generate a verifiable challenge containing location/time data, allowing the IC's signature to be misused for tracking.

### APPENDIX D — WORKED EXAMPLE: BASIC ACCESS CONTROL
Provides a step-by-step hex dump example of deriving $K_{Enc}$ and $K_{MAC}$ from an MRZ seed, performing the `GET CHALLENGE` and `EXTERNAL AUTHENTICATE` commands, and establishing Secure Messaging.

### APPENDIX E — WORKED EXAMPLE: PASSIVE AUTHENTICATION
Outlines the 7-step process of reading the SOD, verifying the Document Signer Certificate, and comparing Data Group hashes.

### APPENDIX F — WORKED EXAMPLE: ACTIVE AUTHENTICATION
Demonstrates an RSA-based Active Authentication flow using a 1024-bit modulus and SHA-1, including the construction of the message representative and partial recovery.

### APPENDIX G — WORKED EXAMPLE: PACE – GENERIC MAPPING
Provides ECDH and DH based examples for PACE using Generic Mapping. Includes full APDU exchanges (`MSE:Set AT`, `GENERAL AUTHENTICATE`), ephemeral key generation, and mutual authentication tokens.

### APPENDIX H — WORKED EXAMPLE: PACE – INTEGRATED MAPPING
Demonstrates the Integrated Mapping flow, including the pseudo-random function $R_p(s,t)$ and point encoding $f_G$.

### APPENDIX I — WORKED EXAMPLE: PACE – PACE CA MAPPING
Demonstrates PACE with Chip Authentication Mapping (CAM), including the generation and verification of Encrypted Chip Authentication Data.

### APPENDIX J — INSPECTION PROCEDURES
* **J.1 Inspection Procedure for eMRTD Application (LDS1)**: Access IC -> Passive Auth -> Chip/Active Auth -> Terminal Auth (if required) -> Read Data.
* **J.2 Inspection Procedure for Multi-application eMRTDs (LDS2)**: Requires PACE. Access IC -> Verify EF.CardSecurity -> Chip Auth in Master File -> Terminal Auth -> Read/Write Data.

### APPENDIX K — EUROPEAN EXTENDED ACCESS CONTROL
Points out differences between ICAO Doc 9303 Part 11 and the EU's [TR-03110] EAC specification, notably that EU EAC performs Chip and Terminal Authentication strictly within the eMRTD Application, whereas ICAO allows them in the Master File. Defines the `EF.CVCA` file structure for EU compatibility.

---
*— END —*
