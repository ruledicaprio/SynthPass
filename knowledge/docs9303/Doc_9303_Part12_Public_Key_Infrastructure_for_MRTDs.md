# ICAO Doc 9303
## Machine Readable Travel Documents
### Part 12: Public Key Infrastructure for MRTDs

**Eighth Edition, 2021**

Approved by and published under the authority of the Secretary General  
**INTERNATIONAL CIVIL AVIATION ORGANIZATION**  

Published in separate English, Arabic, Chinese, French, Russian and Spanish editions by the  
INTERNATIONAL CIVIL AVIATION ORGANIZATION  
999 Robert-Bourassa Boulevard, Montréal, Quebec, Canada H3C 5H7  
Downloads and additional information are available at [www.icao.int/security/mrtd](https://www.icao.int/security/mrtd)

**Doc 9303, Machine Readable Travel Documents**  
Part 12 — Public Key Infrastructure for MRTDs  
Order No.: 9303P12  
ISBN 978-92-9265-422-1 (print version)  
ISBN 978-92-9275-422-8 (electronic version)  

© ICAO 2021  
All rights reserved. No part of this publication may be reproduced, stored in a retrieval system or transmitted in any form or by any means, without prior permission in writing from the International Civil Aviation Organization.

---

### AMENDMENTS AND CORRIGENDA

| No. | Date | Entered by |
| :--- | :--- | :--- |
| 1 | 14/6/24 | ICAO |

*The designations employed and the presentation of the material in this publication do not imply the expression of any opinion whatsoever on the part of ICAO concerning the legal status of any country, territory, city or area or of its authorities, or concerning the delimitation of its frontiers or boundaries.*

---

## TABLE OF CONTENTS

1. [SCOPE](#1-scope)
2. [OVERVIEW OF THE PUBLIC KEY INFRASTRUCTURE](#2-overview-of-the-public-key-infrastructure)
3. [ROLES AND RESPONSIBILITIES](#3-roles-and-responsibilities)
   - 3.1 eMRTD PKI
   - 3.2 Authorization PKI
4. [KEY MANAGEMENT](#4-key-management)
   - 4.1 eMRTD PKI
   - 4.2 Authorization PKI
5. [DISTRIBUTION MECHANISMS](#5-distribution-mechanisms)
   - 5.1 PKD Distribution Mechanism
   - 5.2 Bilateral Exchange Distribution Mechanism
   - 5.3 Master List Distribution Mechanism
6. [PKI TRUST AND VALIDATION](#6-pki-trust-and-validation)
   - 6.1 eMRTD PKI
   - 6.2 Authorization PKI
7. [CERTIFICATE AND CRL PROFILES](#7-certificate-and-crl-profiles)
   - 7.1 eMRTD PKI
   - 7.2 Authorization PKI
8. [SPOC PROTOCOL](#8-spoc-protocol)
   - 8.1 SPOC Related Structures
   - 8.2 SPOC Protocol Messages
   - 8.3 Web Service
9. [CSCA MASTER LIST STRUCTURE](#9-csca-master-list-structure)
   - 9.1 SignedData Type
   - 9.2 ASN.1 Master List Specification
10. [DEVIATION LIST STRUCTURE](#10-deviation-list-structure)
    - 10.1 SignedData Type
    - 10.2 ASN.1 Specification
11. [REFERENCES (NORMATIVE)](#11-references-normative)
- **APPENDIX A** — Lifetimes
- **APPENDIX B** — Certificate and CRL Profile Reference Text
- **APPENDIX C** — Earlier Certificate Profiles
- **APPENDIX D** — RFC 5280 Validation Compatibility
- **APPENDIX E** — LDS2 Example

---

## 1. SCOPE

Part 12 of Doc 9303 defines the Public Key Infrastructure (PKI) for the eMRTD application. Requirements for issuing States or organizations are specified, including operation of a Certification Authority (CA) that issues certificates and Certificate Revocation Lists (CRLs). Requirements for receiving States and their Inspection Systems validating those certificates and CRLs are also specified.

The Eighth Edition of Doc 9303 incorporates the specifications for Visible Digital Seals (known as VDS) and for the optional Travel Records, Visa Records, and Additional Biometric Applications (known as LDS2) as an extension of the mandatory eMRTD application (known as LDS1).

Doc 9303-12 shall be read in conjunction with:
- Doc 9303-10 — Logical Data Structure (LDS) for Storage of Biometrics and Other Data in the Contactless Integrated Circuit (IC);
- Doc 9303-11 — Security Mechanisms for MRTDs; and
- Doc 9303-13 — Visible Digital Seals.

---

## 2. OVERVIEW OF THE PUBLIC KEY INFRASTRUCTURE

The eMRTD Public Key Infrastructure (PKI) enables the creation and subsequent verification of digital signatures on eMRTD objects, including the Document Security Object (SOD) to ensure the signed data is authentic and has not been modified. Revocation of a certificate, failure of the certification path validation procedure, or failure of digital signature verification does not on its own cause an eMRTD to be considered invalid. Such a failure means that the electronic verification of the integrity and authenticity of the LDS data has failed, and other non-electronic mechanisms could then be used to make that determination.

The eMRTD PKI is much simpler than more generic multi-application PKIs such as the Internet PKI defined in [RFC 5280]. In the eMRTD PKI, each issuing State/Authority establishes a single Certification Authority (CA) that issues all certificates directly to end-entities, including Document Signers. These CAs are referred to as **Country Signing Certification Authorities (CSCAs)**. There are no other CAs in the infrastructure. Receiving States establish trust directly in the keys/certificates of each issuing State or organization’s CSCA.

The eMRTD PKI is based on generic PKI standards including [X.509] and [RFC 5280]. Those base PKI standards define a large set of optional features and complex trust relationships among CAs that are not relevant to the eMRTD application. A profile of those standards, tailored to the eMRTD application, is specified in this part of Doc 9303. Some of the unique aspects of the eMRTD application include:
- there is precisely one CSCA per issuing State;
- certification paths include precisely one certificate (e.g., Document Signer);
- signature verification must be possible 5-10 years after creation;
- CSCA name change is supported; and
- CSCA Link certificates are not processed as intermediate certificates in a certification path.

For VDS and LDS2, the Digital Signature PKI, which provides integrity and authenticity of the data objects, is an extension of the LDS1 PKI. The Signers for VDS and LDS2 are issued by the same CSCA which issues Signers for LDS1. Taken together, this infrastructure is referred to as the eMRTD PKI.

The Digital Signature PKI consists of the following entities:
- **Country Signing CA (CSCA)**;
- **Document Signer Certificates (DSC)** which is used to sign the Document Security Objects (SOD);
- **LDS2 Signer Certificates**, which consists of:
  - LDS2-TS Signer – signs LDS2 Travel Stamps;
  - LDS2-V Signer – signs LDS2 Electronic Visas; and
  - LDS2-B Signer – signs LDS2 Additional Biometrics;
- **Bar Code Signer Certificates (BCSC)**, for which the following two specific types are defined:
  - Visa Signer Certificates (VSC); and
  - Emergency Travel Document Signer Certificates (ESC);
- **Master List Signer Certificates (MSC)** used to sign Master Lists;
- **Deviation List Signer Certificates (DLSC)** used to sign Deviation Lists; and
- **Certificate Revocation List (CRL)**.

All the different certificate types are signed by the same CSCA. The CSCA also signs the CRL, which contains any revoked certificate irrespective of the type of certificate. All the certificates issued under the CSCA are collectively referred to as Signer Certificates.

For LDS2 applications, a separate **Authorization PKI** is defined. The Authorization PKI enables the eMRTD issuing State or organization to control and manage the foreign States that are given authorization to write LDS2 data objects to their eMRTDs and to read those data objects. A foreign State intending to read or write LDS2 data must obtain an authorization certificate directly from the eMRTD issuing State or organization.

The LDS2 authorization PKI consists of the following entities:
- **Country Verifying CAs (CVCAs)**;
- **Document Verifiers (DVs)**;
- **Terminals**; and
- **Single Point of Contact (SPOC)**.

---

## 3. ROLES AND RESPONSIBILITIES

### 3.1 eMRTD PKI
The authenticity and integrity of data stored on eMRTDs is protected by Passive Authentication. This security mechanism is based on digital signatures and consists of the following PKI entities for eMRTD PKI:
- **Country Signing CA (CSCA)**: Each issuing State/Authority establishes a single CSCA as its national trust point in the context of eMRTDs. The CSCA issues public key certificates for one or more (national) Document Signers and optionally for other end-entities such as Master List Signers and Deviation List Signers. The CSCA also issues periodic Certificate Revocation Lists (CRL).
- **Document Signers (DS)**: A Document Signer digitally signs data to be stored on eMRTDs; this signature is stored on the eMRTD in a Document Security Object.
- **LDS2 Signers**: An LDS2 Signer digitally signs LDS2 data objects of one or more types.
- **Bar Code Signer (BCS)**: A Bar Code Signer digitally signs the data (header and message) encoded in the bar code. The signature is also stored in the bar code. This document specifies two use cases: Visa and Emergency Travel Documents.
- **Inspection Systems (IS)**: An Inspection System verifies the digital signature, including certification path validation to verify the authenticity and integrity of the electronic data stored on the eMRTD as part of Passive Authentication.
- **Master List Signers**: An optional entity that digitally signs a list of CSCA certificates (domestic and foreign) in support of the bilateral distribution mechanism for CSCA certificates.
- **Deviation List Signers**: Used to sign Deviation Lists.

### 3.2 Authorization PKI
The LDS2 application is written to the contactless IC of an eMRTD by the Issuing State or organization at the time of personalization. Before another State can write LDS2 objects to that contactless IC, it MUST obtain authorization from the Issuing State or organization to do so.
- **Country Verifying Certificate Authority (CVCA)**: Each issuing State or organization that allows LDS2 data to be added to its eMRTDs MUST set up a single CVCA. This CVCA is a Certification Authority (CA) that is the trust anchor for the authorization PKI of that State or organization and covers all LDS2 applications.
- **Document Verifier (DV)**: A CA that, as part of an organizational unit, manages a group of terminals and issues authorization certificates to those terminals.
- **Terminal/Inspection System**: Within the context of the authorization PKI, a terminal is the entity that accesses the contactless IC of an eMRTD and writes a digitally signed LDS2 data object, or reads an LDS2 data object.
- **Single Point of Contact (SPOC)**: Each State that participates in the LDS2 authorization PKI MUST set up a single SPOC. This SPOC is the interface that is used for all communication between the CVCA of one State with the DVs in another State.

---

## 4. KEY MANAGEMENT

### 4.1 eMRTD PKI
Issuing States or organizations SHALL have at least two key pair types:
- Country Signing CA key pair; and
- Document Signer key pair.

Issuing States or organizations MAY have additional key pair types:
- Master List Signer key pair;
- Deviation List Signer key pair;
- LDS2 Signer key pair;
- SPOC client key pair;
- SPOC server key pair; and
- Visa Signer key pair / Emergency Travel Document Signer key pair.

**Table 1. Key Usage and Validity**
| Entity | Use of Private Key | Public Key Validity (assuming 10-year valid passports) |
| :--- | :--- | :--- |
| Country Signing CA | 3-5 years | 13-15 years |
| Document Signer | Up to 3 months | approx. 10 years |
| LDS2-TS Signer | 1-2 years | 10 years + 3 months |
| LDS2-V Signer | 1-2 years | 10 years + 3 months |
| LDS2-B Signer | 1-2 years | 10 years + 3 months |
| SPOC Client | Not Specified | 6-18 months |
| SPOC Server | Not Specified | 6-18 months |
| Visa Bar Code Signer | 1-2 years | Private Key Usage Time + Validity of Visa |
| Emergency Travel Document Bar Code Signer | 1 year + 2 months | Private Key Usage Time + ETD validity timeframe |

#### 4.1.1 Document Signer Keys and Certificates
The usage period of a Document Signer private key is much shorter than the validity period of the DS certificate for the corresponding public key. It is RECOMMENDED that the maximum period any Document Signer private key is used to sign eMRTDs be three months. Once the last document signed with a given private key has been produced, it is RECOMMENDED that issuing States or organizations erase the private key in an auditable and accountable manner.

#### 4.1.2 LDS2 Signer Keys and Certificates
LDS2 Signer key pairs are similar to Document Signer key pairs. The certificates MUST remain valid for the lifetime of the eMRTD or the signed LDS2 object (whichever is longest). Because signed data objects will be written to eMRTDs from various States, these certificates MUST be valid for at least the duration of the longest eMRTD lifetime (i.e., 10 years).

#### 4.1.3 Bar Code Signer Keys and Certificates
To follow best practices, it is RECOMMENDED that only a limited number of signing keys (a lower one-digit number) is used in parallel to create signatures for digital seals. In order to facilitate the handling of the corresponding certificates, the number of published signature validation keys MUST be limited to five signature keys per year.

#### 4.1.4 CSCA Keys and Certificates
The usage period of a CSCA private key is much shorter than the validity period of the CSCA certificate. It is RECOMMENDED that an issuing State or organization’s CSCA key pair be replaced every three to five years. Issuing States or organizations MUST notify receiving States that a CSCA key rollover is planned 90 days in advance. When a CSCA key rollover occurs, a certificate MUST be issued that links the new key to the old key (CSCA Link certificate).

#### 4.1.5 Certificate Revocation
All CSCAs MUST produce periodic revocation information in the form of a Certificate Revocation List (CRL). CSCAs MUST issue at least one CRL every 90 days, even if no certificates have been revoked since the previous CRL was issued. CRLs MAY be issued more frequently than every 90 days but not more frequently than every 48 hours. If a certificate is revoked, a CRL indicating that revocation MUST be distributed within 48 hours.

#### 4.1.6 Cryptographic Algorithms
For use in their CSCA, Signing keys, and Document Security Objects, issuing States or organizations SHALL support one of the following:
- **RSA**: [RFC 4055] specifying RSASSA-PSS and RSASSA-PKCS1_v15.
- **Digital Signature Algorithm (DSA)**: [FIPS 186-4].
- **Elliptic Curve DSA (ECDSA)**: [X9.62] or [ISO/IEC 15946]. The elliptic curve domain parameters used to generate the ECDSA key pair MUST be described explicitly in the parameters of the public key (no named curves, no implicit parameters) and MUST include the optional co-factor. ECPoints MUST be in uncompressed format.
- **Hashing Algorithms**: SHA-224, SHA-256, SHA-384, and SHA-512 are the only permitted hashing algorithms.

#### 4.1.7 Cryptographic Algorithms for LDS2 Signer Certificates
Because LDS2 certificates and signed objects are stored on the contactless IC, they need to be as compact as possible. Therefore, LDS2 Signers MUST use ECDSA, irrespective of the algorithm used in the CSCA and Document Signing keys.

#### 4.1.8 Cryptographic Algorithms for Visa or ETD Signer Certificates
The visa or ETD Signers MUST use ECDSA, irrespective of the algorithm used in the CSCA and Document Signing keys.

### 4.2 Authorization PKI
Issuing States or organizations that implement LDS2 SHALL have the following key pair types:
- Country Verifying CA (CVCA) Key Pair;
- Document Verifier (DV) Key Pair; and
- Terminal Key Pair.

There is no revocation mechanism for CVCA, DV, or terminal certificates. Therefore, their validity periods are much shorter than the X.509 certificate types.

**Table 2. Key Usage Card-Verifiable Certificate Validity**
| Entity | Public Key Validity |
| :--- | :--- |
| CVCA | 6 months – 3 years |
| DV | 2 weeks – 3 months |
| Terminal | 1 day – 1 month |

#### 4.2.1 Cryptographic Algorithms for Terminal Authentication
For Terminal Authentication, either RSA or ECDSA MAY be used. Details are provided in Doc 9303-11.

#### 4.2.2 Cryptographic Algorithms for SPOC
The TLS Encryption Suites to be used for the SPOC protocol are listed below. Both the server and the client side SHALL support RSA and ECDSA-based authentication.

**Table 3. TLS Encryption Suites**
| Cipher Suite | Certificate and Key Exchange Algorithm |
| :--- | :--- |
| TLS_RSA_WITH_AES_128_CBC_SHA | RSA |
| TLS_DHE_RSA_WITH_AES_128_CBC_SHA | DHE_RSA |
| TLS_RSA_WITH_AES_256_CBC_SHA | RSA |
| TLS_DHE_RSA_WITH_AES_256_CBC_SHA | DHE_RSA |
| TLS_ECDHE_ECDSA_WITH_AES_128_CBC_SHA | ECDHE_ECDSA |
| TLS_ECDHE_ECDSA_WITH_AES_256_CBC_SHA | ECDHE_ECDSA |

---

## 5. DISTRIBUTION MECHANISMS

For eMRTD PKI, the PKI objects need to be distributed to the receiving States. Distribution of these objects does NOT establish trust in those objects. Mechanisms for establishing trust are specified in Section 6.1.

**Table 4. Distribution of PKI Objects**
| Object | Contactless IC | SPOC | Bilateral | PKD | Deviation List | Master List | Notes |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| CSCA Certificates | | | Y (primary) | | | Y (secondary) | |
| Document Signer Certificates | Y (primary) | | | Y (secondary) | | | Certificates written at same time SOD is written |
| LDS2 Signer certificates | Y | | | | | | Certificates written at same time signed object is written |
| CVCA Initial Certificate | Y | | | | | | Certificate written at eMRTD personalization time |
| CVCA Link Certificates | Y | Y | | | | | Certificates distributed to DVs via SPOC and CVCA Trust Anchor updated on contactless IC |
| DV Certificates | | Y | | | | | Distributed only to subject DV |
| CRLs (Null and Non-null) | | | Y (secondary) | Y (primary) | | | CRLs issued by CSCA include revocation information relevant to LDS2 PKI objects |
| Master List Signer Certificates | | | | | | Y | |
| Bar Code Signer Certificates | | | Y (secondary) | Y (primary) | | | Bar Code Signers are not encoded in the Bar Code and hence distribution must be ensured |
| Master Lists | | | Y | Y | | | |
| Deviation List Signer Certificates | | | | | Y | | |

### 5.1 PKD Distribution Mechanism
ICAO provides a Public Key Directory (PKD) service. This service SHALL accept PKI objects, including certificates, CRLs, and Master Lists, from PKD participants, store them in a directory, and make them accessible to all receiving States. Read access to all certificates, CRLs, and Master Lists published in the PKD SHALL be available to PKD participants and non-participants. Access control SHALL NOT be implemented for PKD read access.

### 5.2 Bilateral Exchange Distribution Mechanism
For CRLs and CSCA certificates (CCSCA), the primary distribution channel is bilateral exchange between issuing States or organizations and receiving States. Technologies may include diplomatic courier/pouch, email exchange, or download from a website/LDAP server associated with the issuing CSCA.

### 5.3 Master List Distribution Mechanism
Master Lists are a supporting technology for the bilateral distribution scheme. A Master List is a digitally signed list of the CSCA certificates that are “trusted” by the receiving State or organization that issued the Master List. CSCA self-signed Root certificates and CSCA Link certificates may be included in a Master List.

---

## 6. PKI TRUST AND VALIDATION

### 6.1 eMRTD PKI
In the eMRTD PKI environment, the Inspection Systems in receiving States act in the role of PKI relying parties. Successful verification of the digital signature on the Document Security Object of an eMRTD ensures the authenticity and integrity of the data stored on the contactless IC of that eMRTD.

#### 6.1.1 Trust Anchor Management
A Trust Anchor must be established that can be used to anchor the validation procedure for a given Document Signer, Master List Signer, Deviation List Signer, or other type of certificate. Each Trust Anchor is comprised of a trusted public key and associated metadata, including the trusted public key and any associated key parameters, the public key algorithm, the name of the key owner, and the value of the SubjectAltName extension of the CSCA certificate containing the ICAO assigned three-letter code.

For the initial public key obtained from a CSCA, trust MUST be established through an out-of-band mechanism (e.g., phone or email). Once an initial Trust Anchor is established for a given CSCA, the process could be simplified for subsequent keys for that same CSCA via CSCA Link certificates.

#### 6.1.2 Certificate/CRL Validation and Revocation Checking
As part of the process of verifying the authenticity and integrity of data objects in the eMRTD application, a Receiving State:
1. validates the certificate used to verify the signature on the data object;
2. validates the CRL that is used to check the revocation status of the certificate in question; and
3. processes the CRL to verify the revocation status of the certificate in question.

### 6.2 Authorization PKI
For Authorization PKI, the Trust Anchor and Validation is handled differently.

#### 6.2.1 Validation of Card Verifiable Certificates
For DV and terminal certificates in the authorization PKI, the Trust Anchor is the most recent public key of the CVCA of the State that issued the eMRTD. The initial Trust Anchor SHALL be stored securely in the eMRTD contactless IC in the production or (pre-)personalization phase. As the key pair used by the CVCA changes over time, CVCA Link Certificates are produced. The eMRTD contactless IC MUST internally update its Trust Anchor(s) according to received valid link certificates. Due to the scheduling of CVCA Link Certificates, at most two CVCA Trust Anchors will be stored on the contactless IC at any one time.

---

## 7. CERTIFICATE AND CRL PROFILES

Certificate profiles are defined for both the eMRTD PKI and the Authorization PKI. All certificates and CRLs MUST be produced in Distinguished Encoding Rule (DER) format to preserve the integrity of the signatures within them.

### 7.1 eMRTD PKI
The profiles use the following terminology for presence requirements: `m` (mandatory), `x` (do not use), `o` (optional), `c` (conditional). For criticality: `c` (critical), `nc` (non-critical).

**Table 5. Certificate Fields Profile**
| Certificate Component | Presence | Comments |
| :--- | :--- | :--- |
| Certificate | m | |
| TBSCertificate | m | See Table 6 |
| signatureAlgorithm | m | Value inserted here dependent on algorithm selected |
| signatureValue | m | Value inserted here dependent on algorithm selected |
| version | m | MUST be v3 |
| serialNumber | m | MUST be positive integer and maximum 20 Octets. MUST use 2’s complement encoding. |
| issuer | m | countryName and serialNumber, if present, MUST be PrintableString. countryName MUST be Upper Case. |
| validity | m | MUST terminate with Zulu (Z). Seconds element MUST be present. Dates through 2049 MUST be in UTCTime. Dates in 2050 and beyond MUST be in GeneralizedTime. |
| subject | m | countryName and serialNumber, if present, MUST be PrintableString. countryName MUST be Upper Case. countryName in issuer and subject fields MUST match. |
| subjectPublicKeyInfo | m | |
| issuerUniqueID | x | |
| subjectUniqueID | x | |
| extensions | m | See Table 6 on which extensions should be present. Default values for extensions MUST NOT be encoded. |

**Table 6. Certificate Extensions Profile**
| Extension name | CSCA Self-Signed Root | CSCA Link | Document Signer | Master List Signer & Deviation List Signer | Communication | Comments |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| AuthorityKeyIdentifier | o/nc | m/nc | m/nc | m/nc | m/nc | |
| SubjectKeyIdentifier | m/nc | m/nc | o/nc | o/nc | o/nc | |
| KeyUsage | m/c | m/c | m/c | m/c | m/c | `keyCertSign` and `cRLSign` MUST be asserted for CSCA; `digitalSignature` for Document Signers. |
| PrivateKeyUsagePeriod | m/nc | m/nc | m/nc | o/nc | o/nc | At least one of notBefore or notAfter MUST be present. |
| CertificatePolicies | o/nc | o/nc | o/nc | o/nc | o/nc | |
| PolicyMappings | x | x | x | x | x | See Note 1 |
| SubjectAltName | m/nc | m/nc | m/nc | m/nc | m/nc | See 7.1.1.2 |
| IssuerAltName | m/nc | m/nc | m/nc | m/nc | m/nc | See 7.1.1.2 |
| SubjectDirectoryAttributes | x | x | x | x | x | |
| Basic Constraints | m/c | m/c | x | x | x | `cA` MUST be `TRUE` for CSCA, `PathLenConstraint` MUST always be ‘0’. |
| NameConstraints | x | x | x | x | x | See Note 1 |
| PolicyConstraints | x | x | x | x | x | See Note 1 |
| ExtKeyUsage | x | x | x | m/c | m/c | See 7.1.1.3 |
| CRLDistributionPoints | m/nc | m/nc | m/nc | m/nc | o/nc | MUST be ldap, http or https. See 7.1.1.4 |
| FreshestCRL | x | x | x | x | x | See Note 2 |
| privateInternetExtensions | o/nc | o/nc | o/nc | o/nc | o/nc | See Note 3 |
| NameChange | o/nc | o/nc | x | x | x | See 7.1.1.5 |
| DocumentType | x | x | m/nc | x | x | See 7.1.1.6 |
| Netscape Certificate Type | x | x | x | x | x | See Note 4 |

*Note 1:* The extension, by definition, can only appear in intermediate CA certificates. Intermediate CA certificates are not used in the eMRTD PKI. Therefore this extension is prohibited.
*Note 2:* The freshest CRL extension is used to point to a delta CRL. Delta CRLs are not supported in the eMRTD PKI. Therefore this extension is prohibited.
*Note 3:* There are two Private Internet Extensions (Authority Information Access and Subject Information Access) defined in RFC 5280. These extensions are not required in the eMRTD PKI. However, as they do not impact interoperability, and are non-critical, they may optionally be included.
*Note 4:* The Netscape Certificate Type extension can be used to limit the purposes for which a certificate can be used. The extKeyUsage and basicConstraints extensions are now the standard extensions for those purposes. Because of the potential conflict, the Netscape extension is prohibited.

#### 7.1.1.1 Issuer and Subject Field Requirements
- **countryName**: MUST be present. The value contains a country code that MUST follow the format of two-letter country codes, specified in Doc 9303-3.
- **commonName**: MUST be present.

#### 7.1.1.2 Issuer and Subject Alternative Name Requirements
In the eMRTD application, alternative names serve the following two functions:
1. To provide contact information for the subject and/or issuer of the certificate (e.g., rfc822Name, dNSName, or uniformResourceIdentifier).
2. To provide a directory string made of ICAO assigned country codes. For this purpose, certificates issued using this profile MUST additionally include a directory name that is constructed as follows:
   - `localityName` that contains the ICAO country code as it appears in the MRZ; and
   - if this country code does not uniquely define the issuing State or organization, the attribute `stateOrProvinceName` SHALL be used to indicate the ICAO assigned three-letter code for the issuing State or organization.

#### 7.1.1.3 Extended Key Usage Extension Requirements
- Master List Signer certificates: OID `2.23.136.1.1.3`
- Deviation List Signer certificates: OID `2.23.136.1.1.8`

#### 7.1.1.4 CRL Distribution Points Extension Requirements
For CRLs submitted to the PKD, PKD participants MAY include two URL values for their CRL using the following template (replace “Country Code” with the issuing State or organization ICAO assigned three-letter code):
- `https://pkddownload1.icao.int/CRLs/CountryCode.crl`
- `https://pkddownload2.icao.int/CRLs/CountryCode.crl`

#### 7.1.1.5 Name Change Extension
When a CSCA key rollover occurs, a certificate MUST be issued that links the old public key to the new public key. If a name change is necessary, this MUST be conveyed to relying parties through the issuance of a CSCA Link certificate where the issuer field contains the old name and the subject field contains the new name. Certificates that convey both a CSCA name change and a key rollover for that CSCA MUST include the NameChange extension.

ASN.1 for Name Change extension:
```asn1
nameChange EXTENSION ::= {
    SYNTAX NULL
    IDENTIFIED BY id-icao-mrtd-security-extensions-nameChange
}
id-icao-mrtd-security-extensions OBJECT IDENTIFIER ::= {id-icao-mrtd-security 6}
id-icao-mrtd-security-extensions-nameChange OBJECT IDENTIFIER ::= {id-icao-mrtd-security-extensions 1}
```

#### 7.1.1.6 Document Type Extension
The DocumentType extension MUST be used to indicate the document types, as they appear in the MRZ, that the corresponding Document Signer is allowed to produce. This extension MUST always be set to non-critical.

```asn1
documentTypeList EXTENSION ::= {
    SYNTAX DocumentTypeListSyntax
    IDENTIFIED BY id-icao-mrtd-security-extensions-documentTypeList
}
DocumentTypeListSyntax ::= SEQUENCE {
    version DocumentTypeListVersion,
    docTypeList SET OF DocumentType
}
DocumentTypeListVersion ::= INTEGER {v0(0)}
DocumentType ::= PrintableString (SIZE(1..2))
id-icao-mrtd-security-extensions-documentTypeList OBJECT IDENTIFIER ::= {id-icao-mrtd-security-extensions 2}
```

### 7.1.2 LDS2 Signer Certificate Profile
LDS2 Signer certificates MUST comply with the Document Signer certificate profile defined in 7.1.1 with the following exceptions:
- **Subject Field**:
  - `countryName`: MUST be present (two-letter country code).
  - `commonName`: MUST be present. The value in this attribute MUST NOT exceed 9 characters in length.
  - Other attributes MUST NOT be included.
- **Certificate Extensions**: LDS2 Signer certificates MUST contain the certificate extensions identified in Table 7 below. All other certificate extensions MUST NOT be included.

**Table 7. Mandatory Certificate Extensions for LDS2**
| Extension name | Presence | Criticality | Comments |
| :--- | :---: | :---: | :--- |
| AuthorityKeyIdentifier | m | nc | |
| ExtKeyUsage | m | c | See Note below |

*Note:* The EKU extension for each LDS2 Signer certificate type MUST be populated as indicated below:
- LDS2 Travel Stamp Signer (LDS2-TS) certificates: `id-icao-tsSigner` OBJECT IDENTIFIER ::= `{ id-icao-lds2Signer 1 }`
- LDS2 Visa Signer (LDS2-V) certificates: `id-icao-vSigner` OBJECT IDENTIFIER ::= `{ id-icao-lds2Signer 2 }`
- LDS2 Biometrics Signer (LDS2-B) certificates: `id-icao-bSigner` OBJECT IDENTIFIER ::= `{ id-icao-lds2Signer 3 }`

### 7.1.3 Bar Code Signer Certificate Profile
The Bar Code Signer certificates MUST comply with the LDS2 Signer certificate profile. Since Bar Code Signer certificates serve a different role than LDS2 certificates, their profile deviates in some respects.
- **Subject Field**:
  - `commonName`: MUST be present. MUST consist of two upper case characters, printableString format, that uniquely define the Bar Code Signer within one country, and MUST match the letters 3 and 4 of the Signer Identifier in the bar code.
  - `countryName`: MUST consist of the two-letter country code of the Bar Code Signer, uppercase characters, printableString format, and MUST match letters 1 and 2 of the Signer Identifier in the bar code.
  - Other attributes MUST NOT be included.

**Table 8. Allowed Extensions for Bar Code Signer Certificates**
| Extension name | Presence | Criticality | Comments |
| :--- | :---: | :---: | :--- |
| AuthorityKeyIdentifier | m | nc | |
| DocumentType | o | | This extension indicates the document type, which the Bar Code Signer is allowed to produce |
| ExtKeyUsage | m | c | See note below |

*Note:* The EKU extension for each Bar Code Signer certificate type MUST be populated as indicated below:
- `id-icao-vdsSigner` OBJECT IDENTIFIER ::= `{ id-icao-mrtd-security-vds 1 }`

### 7.1.4 CRL Profile
**Table 9. CRL Fields Profile**
| Certificate List Component | CSCA CRL | Comments |
| :--- | :--- | :--- |
| CertificateList | m | |
| tBSCertList | m | See Table 10 |
| signatureAlgorithm | m | Value inserted here dependent on algorithm selected |
| signatureValue | m | Value inserted here dependent on algorithm selected |
| Version | m | MUST be v2 |
| Signature | m | value inserted here MUST be the same as that in signatureAlgorithm component of CertificateList sequence |
| Issuer | m | countryName and serialNumber, if present, MUST be PrintableString. Other attributes that have DirectoryString syntax MUST be either PrintableString or UTF8String. countryName MUST be Upper Case. |
| thisUpdate | m | MUST terminate with Zulu (Z). Seconds element MUST be present. Dates through 2049 MUST be in UTCTime. Dates in 2050 and beyond MUST be in GeneralizedTime. |
| nextUpdate | m | MUST terminate with Zulu (Z). Seconds element MUST be present. Dates through 2049 MUST be in UTCTime. Dates in 2050 and beyond MUST be in GeneralizedTime. |
| revokedCertificates | c | SHALL be present if there are revoked certificates. If there are no revoked certificates it SHALL NOT be present. If present, MUST NOT be empty. |
| crlExtensions | m | See Table 10 on which extensions should be present. Default values for extensions MUST NOT be encoded. |

**Table 10. CRL and CRL Entry Extensions Profile**
| Extension Name | CSCA CRL | Criticality | Comments |
| :--- | :---: | :---: | :--- |
| **CRL Extensions** | | | |
| authorityKeyIdentifier | m | nc | This MUST be the same value as the subjectKeyIdentifier field in the CRL issuer’s certificate. |
| issuerAlternativeName | o | nc | See Note 1 |
| cRLNumber | m | nc | MUST be non-negative integer and maximum 20 Octets. MUST use 2’s complement encoding. |
| deltaCRLIndicator | x | | |
| issuingDistributionPoint | x | | |
| freshestCRL | x | | |
| **CRL Entry Extensions** | | | |
| reasonCode | x | | |
| holdInstructionCode | x | | |
| invalidityDate | x | | |
| certificateIssuer | x | | |
| other private extensions | o | nc | |

*Note 1:* If a CSCA has undergone a name change, this extension MAY be included in CRLs issued following the CSCA name change. If present, the value(s) in this extension MUST be identical to the issuer field of certificates issued by the CSCA under that previous name.

---

### 7.2 Authorization PKI
The authorization PKI includes X.509 certificates for SPOC and card-verifiable certificates for CVCA, DV, and terminals.

#### 7.2.1 SPOC Certificate Profile
LDS2 SPOC certificates (client and server) MUST comply with the communication certificate profile defined in Section 7.1, with the following restrictions:
- **Issuer Field**: SPOC certificates are issued either by the CSCA or a separate CA setup specifically to issue SPOC certificates.
- **Subject Field**:
  - `countryName`: MUST be present (two-letter country code).
  - `commonName`: MUST be present. For SPOC TLS client certificates, the value SHOULD be “SPOC TLS client”. For SPOC TLS server certificates, the value SHOULD be “SPOC TLS server”.
- **Extended Key Usage Extensions**:
  - SPOC client certificates: OID is `2.23.136.1.1.10.1`;
  - SPOC server certificates: OID is `2.23.136.1.1.10.2`.
- **CRL Distribution Point Extensions**: This extension is mandatory in SPOC client and server certificates.

#### 7.2.2 CVCA, DV and Terminal Certificate Profiles
CVCA Link Certificates, DV Certificates, and Terminal Certificates are to be validated by ICs. Due to the computational restrictions of those chips, the certificates MUST be in a card-verifiable format (CV certificates).

**Table 11. CV Certificate Profile**
| Data Object | Certificate Presence |
| :--- | :---: |
| CV Certificate | m |
| Certificate Body | m |
| Certificate Profile Identifier | m |
| Certification Authority Reference | m |
| Public Key | m |
| Certificate Holder Reference | m |
| Certificate Holder Authorization Template | m |
| Certificate Effective Date | m |
| Certificate Expiration Date | m |
| Certificate Extensions | o |
| Signature | m |

##### 7.2.2.1 Certificate Profile Identifier
The version of the profile is indicated by the Certificate Profile Identifier. Version 1 SHALL be used and is identified by a value of 0.

##### 7.2.2.2 Certificate Authority Reference and Certificate Holder Reference
Each CV Certificate MUST contain two public key references (a Certificate Holder Reference and a Certification Authority Reference).
The Certificate Holder Reference SHALL consist of the following concatenated elements: Country Code, Holder Mnemonic, and Sequence Number.

**Table 12. Certificate Holder Reference**
| Encoding | Length |
| :--- | :--- |
| Country Code (Doc 9303-3) | 2F |
| Holder Mnemonic (ISO/IEC 8859-1) | 9V |
| Sequence Number (ISO/IEC 8859-1) | 5F |

##### 7.2.2.3 Public Key
CVCA self-signed certificates MUST contain domain parameters. CVCA Link certificates MAY contain domain parameters, except in the case where domain parameters have changed. In such cases, the Link certificates MUST contain the new domain parameters. DV and Terminal certificates MUST NOT contain domain parameters. The domain parameters of DV and terminal public keys SHALL be inherited from the respective CVCA public key.

##### 7.2.2.4 Certificate Holder Authorization Template
The role and authorization of the certificate holder SHALL be encoded in the Certificate Holder Authorization Template. This template is a sequence that consists of:
a) an object identifier that specifies the terminal type and the format of the template; and
b) a discretionary data object that encodes the relative authorization, i.e., the role and authorization of the certificate holder relative to the certification authority.

##### 7.2.2.5 Certificate Effective Date and Certificate Expiration Date
The combination of these two dates indicate the validity period of the certificate. The Certificate Effective Date MUST be the date of the certificate generation.

##### 7.2.2.6 Certificate Extensions (Authorization Extensions)
Authorization extensions MAY be included in CVCA, DV and terminal certificates. These extensions convey authorizations additional to those in the Certificate Holder Authorization Template in the certificate.

**Table 13. Certificate Extensions**
| Data Object |
| :--- |
| Certificate Extensions |
| Discretionary Data Template |
| Object Identifier |
| Context Specific Data Object |
| Discretionary Data Template |
| Object Identifier |
| Context Specific Data Object |
| ... |

##### 7.2.2.7 Signature
The signature on the certificate SHALL be created over the encoded certificate body (i.e., including tag and length). The Certification Authority Reference SHALL identify the public key to be used to verify the signature.

#### 7.2.3 Data Objects
An overview of the tags, lengths and values of the data objects used in CVCA, DV and terminal certificates is provided below.

**Table 14. Overview of Data Objects (sorted by tag)**
| Name | Tag | Len | Value | Comment |
| :--- | :--- | :--- | :--- | :--- |
| Object Identifier | 0x06 | V | Object Identifier | – |
| Certification Authority Reference | 0x42 | 16V | Character String | Identifies the public key of the issuing certification authority in a certificate. |
| Discretionary Data | 0x53 | V | Octet String | Contains arbitrary data. |
| Certificate Holder Reference | 0x5F20 | 16V | Character String | Associates the public key contained in a certificate with an identifier. |
| Certificate Expiration Date | 0x5F24 | 6F | Date | The date after which the certificate expires. |
| Certificate Effective Date | 0x5F25 | 6F | Date | The date of the certificate generation. |
| Certificate Profile Identifier | 0x5F29 | 1F | Unsigned Integer | Version of the certificate and certificate request format. |
| Signature | 0x5F37 | V | Octet String | Digital signature produced by an asymmetric cryptographic algorithm. |
| Certificate Extensions | 0x65 | V | Sequence | Nests certificate extensions. |
| Authentication | 0x67 | V | Sequence | Contains authentication-related data objects. |
| Discretionary Data Template | 0x73 | V | Sequence | Nests arbitrary data objects. |
| CV Certificate | 0x7F21 | V | Sequence | Nests certificate body and signature. |
| Public Key | 0x7F49 | V | Sequence | Nests the public key value and the domain parameters. |
| Certificate Holder Authorization Template | 0x7F4C | V | Sequence | Encodes the role of the certificate holder (i.e. CVCA, DV, Terminal) and assigns read/write access rights. |
| Certificate Body | 0x7F4E | V | Sequence | Nests data objects of the certificate body. |

*(F: fixed length, V: variable length)*

##### 7.2.3.1 Encoding of Values
- **Unsigned Integers**: An unsigned integer SHALL be converted to an octet string using the binary representation of the integer in big-endian format. The minimum number of octets SHALL be used, i.e., leading octets of value 0x00 MUST NOT be used.
- **Elliptic Curve Points**: The conversion of elliptic curve points to octet strings is specified in [TR-03111]. The uncompressed format SHALL be used.
- **Dates**: A date is encoded in 6 digits “d1...d6” in the format YYMMDD using timezone GMT. It is converted to an octet string “o1...o6” by encoding each digit dj to an octet oj as unpacked BCDs (1 ≤ j ≤ 6).
- **Character Strings**: A character string “c1...cn” is a concatenation of n characters cj with 1 ≤ j ≤ n. It SHALL be converted to an octet string “o1...on” by converting each character cj to an octet oj using the ISO/IEC 8859-1 character set.
- **Octet Strings**: An octet string “o1...on” is a concatenation of n octets oj with 1 ≤ j ≤ n. Every octet oj consists of 8 bits.
- **Object Identifiers**: An object identifier “i1.i2...in” is encoded as an ordered list of n unsigned integers ij with 1 ≤ j ≤ n. It SHALL be converted to an octet string “o1...on−1” using the procedure defined in [X.690].
- **Sequences**: A sequence “D1...Dn” is an ordered list of n data objects Dj with 1 ≤ j ≤ n. The sequence SHALL be converted to a concatenated list of octet strings “O1...On” by DER encoding each data object Dj to an octet string Oj.

##### 7.2.3.2 Encoding of Public Key Data Objects
A public key data object contains a sequence of an object identifier and several context specific data objects.

**Table 15. RSA Public Key**
| Data Object | Abbrev | Tag | Type | CV Certificate |
| :--- | :--- | :--- | :--- | :---: |
| Object Identifier | | 0x06 | Object Identifier | m |
| Composite Modulus | n | 0x81 | Unsigned Integer | m |
| Public Exponent | e | 0x82 | Unsigned Integer | m |

**Table 16. EC Public Key**
| Data Object | Abbrev | Tag | Type | CV Certificate |
| :--- | :--- | :--- | :--- | :---: |
| Object Identifier | | 0x06 | Object Identifier | m |
| Prime Modulus | p | 0x81 | Unsigned Integer | c |
| First coefficient | a | 0x82 | Unsigned Integer | c |
| Second coefficient | b | 0x83 | Unsigned Integer | c |
| Base point | G | 0x84 | Elliptic Curve Point | c |
| Order of the point | r | 0x85 | Unsigned Integer | c |
| Public point | Y | 0x86 | Elliptic Curve Point | m |
| Cofactor | f | 0x87 | Unsigned Integer | c |

---

## 8. SPOC PROTOCOL

The Single Point of Contact (SPOC) is the only interface exposed by a State for key management operations with foreign States for the LDS2 authorization PKI. The SPOC protocol is the key management protocol for operations between CVCAs and DVs in different States.

### 8.1 SPOC Related Structures
**Table 17. CV Certificate Request Profile**
| Data Object | Certificate Presence |
| :--- | :---: |
| Authentication | c |
| CV Certificate | m |
| Certificate Body | m |
| Certificate Profile Identifier | m |
| Certification Authority Reference | r |
| Public Key | m |
| Certificate Holder Reference | m |
| Signature | m |
| Certification Authority Reference | c |
| Signature | c |

#### 8.1.1.1 Certificate Profile Identifier
The version is 1, identified by a value of 0.

#### 8.1.1.2 Certification Authority Reference
The Certification Authority Reference SHOULD be used to inform the certification authority about the private key that is expected by the applicant to be used to sign the certificate.

#### 8.1.1.3 Public Key
Certificate Requests MUST always contain domain parameters.

#### 8.1.1.4 Certificate Holder Reference
The Certificate Holder Reference is used to identify the public key contained in the request and the resulting certificate.

#### 8.1.1.5 Signature(s)
A certificate request may have up to two signatures; an inner signature and an outer signature:
- **Inner Signature (REQUIRED)**: The certificate body is self-signed, i.e., the inner signature SHALL be verifiable with the public key contained in the certificate request.
- **Outer Signature (CONDITIONAL)**: 
  - The signature is OPTIONAL if an entity applies for the initial certificate.
  - The signature is REQUIRED if an entity applies for a successive certificate. In this case, the request MUST be additionally signed by the applicant using a recent key pair previously registered with the receiving certification authority.

### 8.2 SPOC Protocol Messages
#### 8.2.1 Request Certificate Message
**Intended Use**: The RequestCertificate message is used by a SPOC for requesting the generation of a new certificate for one of its DVs from a foreign CVCA.
**Input Parameters**:
- `callerID`: (Mandatory) Two-letter country code according to Doc 9303-3.
- `messageID`: (Mandatory) Identification of the message.
- `certReq`: (Mandatory) The actual certificate request.
**Output Parameters**:
- `certificateSeq`: (Conditional) Contains the result (one or more certificates) after processing this message.
**Return Codes**: `ok_cert_available`, `ok_reception_ack`, `failure_inner_signature`, `failure_outer_signature`, `failure_syntax`, `failure_request_not_accepted`, `failure_request_syntax`, `failure_expired`, `failure_domain_parameters`, `failure_internal_error`.

#### 8.2.2 Send Certificates Message
**Intended Use**: The SendCertificates message is used by a SPOC to send the new certificate or certificate chain to the requesting SPOC.
**Input Parameters**:
- `callerID`: (Mandatory)
- `messageID`: (Conditional)
- `statusInfo`: (Mandatory) e.g., `new_cert_available_notification`, `ok_cert_available`, `failure_inner_signature`, etc.
- `certificateSeq`: (Conditional)

#### 8.2.3 Get CA Certificates Message
**Intended Use**: This message is sent by a SPOC to a foreign SPOC in order to get all valid CVCA certificates (link certificates and self-signed certificates) of that State.
**Input Parameters**: `callerID` (Mandatory), `messageID` (Mandatory).
**Output Parameters**: `certificateSeq` (Conditional).
**Return Codes**: `ok_cert_available`, `ok_reception_ack`, `failure_syntax`, `failure_internal_error`.

#### 8.2.4 General Messages
**Intended Use**: This message is sent by a SPOC to a foreign SPOC in order to send a notification or other general text human-readable message.
**Input Parameters**: `callerID` (Mandatory), `messageID` (Mandatory), `subject` (Mandatory), `body` (Mandatory).
**Return Codes**: `ok`, `failure_syntax`, `failure_internal_error`.

### 8.3 Web Service
The web service interface is the interface for the routine inter-SPOC wire data exchange. The interface SHALL use [SOAP] over [HTTPS] protocol. TLS v1.2 SHALL be used.

#### 8.3.1 SOAP usage
Pure [SOAP] over [HTTPS] SHALL be used to implement the Web Service interfaces. Any other SOAP extensions (e.g., WS-Security, WS-Addressing) SHALL NOT be used. The intermediary SOAP node type SHALL NOT be used.

#### 8.3.2 Security Considerations
The SPOC web service communication SHALL use a secure and authenticated channel. The TLS client SHALL perform following verifications:
- the server certificate SHALL be fully validated according to [RFC5280] including revocation status;
- the server certificate ExtKeyUsage extension MUST be present and SHALL contain the OIDs according to Section 7.2.1 SPOC TLS server certificate; and
- the server certificate subject country SHALL be equal to the value of callerID parameter.

The TLS server SHALL perform following verifications:
- the client SHALL be fully authenticated using a certificate;
- the client certificate SHALL be fully validated according to [RFC5280] including revocation status;
- the client certificate ExtKeyUsage extension MUST be present and SHALL contain the OIDs according to Section 7.2.1 SPOC TLS client certificate; and
- the client certificate subject country SHALL correspond to the intended one.

#### 8.3.3 WSDL for SPOC Web Service Interface
*(Note: Full WSDL XML definition omitted for brevity, but conforms to standard ICAO SPOC SOAP interface definitions including RequestCertificate, SendCertificates, GetCACertificates, and GeneralMessage operations).*

---

## 9. CSCA MASTER LIST STRUCTURE

Master Lists are implemented as instances of the ContentInfo Type, as specified in [RFC 5652]. The ContentInfo MUST contain a single instance of the SignedData Type as profiled below. All Master Lists MUST be produced in DER format to preserve the integrity of the signatures within them.

### 9.1 SignedData Type
**Table 18. Master List**
| Value | Presence | Comments |
| :--- | :---: | :--- |
| SignedData | | |
| Version | m | Value = v3 |
| digestAlgorithms | m | |
| encapContentInfo | m | |
| eContentType | m | id-icao-cscaMasterList |
| eContent | m | The encoded contents of an cscaMasterList |
| Certificates | m | The Master List Signer certificate MUST be included and the CSCA certificate, which can be used to verify the signature in the signerInfos field SHOULD be included. |
| Crls | x | |
| signerInfos | m | It is RECOMMENDED that States only provide 1 signerinfo within this field. |
| SignerInfo | m | |
| Version | m | The value of this field is dictated by the sid field. |
| Sid | m | |
| subjectKeyIdentifier | r | It is RECOMMENDED that this field be supported rather than issuerandSerialNumber. |
| digestAlgorithm | m | The algorithm identifier of the algorithm used to produce the hash value over encapsulatedContent and SignedAttrs. |
| signedAttrs | m | signedAttrs MUST include signing time (see [PKCS#9]). |
| signatureAlgorithm | m | The algorithm identifier of the algorithm used to produce the signature value, and any associated parameters. |
| signature | m | The result of the signature generation process. |
| unsignedAttrs | o | Although this field MAY be included, Receiving States may choose to ignore it. |

*Note:* DigestAlgorithmIdentifiers MUST omit “NULL” parameters, while the SignatureAlgorithmIdentifier (as defined in RFC 3447) MUST include NULL as the parameter if no parameters are present, even when using SHA2 Algorithms in accordance with RFC 5754. Implementations MUST accept DigestAlgorithmIdentifiers with both conditions, absent parameters or with NULL parameters.

### 9.2 ASN.1 Master List Specification
```asn1
CscaMasterList { joint-iso-itu-t(2) international-organization(23) icao(136) mrtd(1) security(1) masterlist(2)}
DEFINITIONS IMPLICIT TAGS ::= BEGIN

IMPORTS
    -- Imports from RFC 5280 [PROFILE], Appendix A.1
    Certificate FROM PKIX1Explicit88 { iso(1) identified-organization(3) dod(6) internet(1) security(5) mechanisms(5) pkix(7) mod(0) pkix1-explicit(18) };

-- CSCA Master List
CscaMasterListVersion ::= INTEGER {v0(0)}

CscaMasterList ::= SEQUENCE {
    version CscaMasterListVersion,
    certList SET OF Certificate
}

-- Object Identifiers
id-icao-cscaMasterList OBJECT IDENTIFIER ::= {id-icao-mrtd-security 2}
id-icao-cscaMasterListSigningKey OBJECT IDENTIFIER ::= {id-icao-mrtd-security 3}

END
```

---

## 10. DEVIATION LIST STRUCTURE

The Deviation List is implemented as a SignedData type, as specified in [RFC 3852]. All Deviation Lists MUST be produced in DER format to preserve the integrity of the signatures within them.

The range of deviations will be bounded by:
- date range (including both the issue and expiry date);
- issuer name and serial number;
- Subject Key Identifier of DSC;
- list of eMRTD numbers.

### 10.1 SignedData Type
**Table 19. Deviation List**
| Value | Presence | Comments |
| :--- | :---: | :--- |
| SignedData | | |
| version | m | Value = v3 |
| digestAlgorithms | m | |
| encapContentInfo | m | |
| eContentType | m | id-icao-DeviationList |
| eContent | m | The encoded contents DeviationList |
| certificates | m | States MUST include the Deviation List Signer certificate and SHOULD include the CSCA certificate, which can be used to verify the signature in the signerInfos field. |
| crls | x | |
| signerInfos | m | It is RECOMMENDED that States provide only 1 signerInfo within this field. |
| SignerInfo | m | |
| version | m | The value of this field is dictated by the sid field. See [RFC 3852] Section 5.3 for rules regarding this field. |
| sid | m | |
| subjectKeyIdentifier | r | It is RECOMMENDED that States support this field over issuerandSerialNumber. |
| digestAlgorithm | m | The algorithm identifier of the algorithm used to produce the hash value over encapsulatedContent and SignedAttrs. |
| signedAttrs | m | signedAttrs MUST include signing time (ref. PKCS#9). |
| signatureAlgorithm | m | The algorithm identifier of the algorithm used to produce the signature value, and any associated parameters. |
| signature | m | The result of the signature generation process. |
| unsignedAttrs | x | |

### 10.2 ASN.1 Specification
```asn1
DeviationList { joint-iso-itu-t(2) international-organization(23) icao(136) mrtd(1) security(1) deviationlist(7)}
DEFINITIONS IMPLICIT TAGS ::= BEGIN

IMPORTS
    -- Imports from RFC 3280 [PROFILE], Appendix A.1
    AlgorithmIdentifier FROM PKIX1Explicit88 { iso(1) identified-organization(3) dod(6) internet(1) security(5) mechanisms(5) pkix(7) mod(0) pkix1-explicit(18) }
    
    -- Imports from RFC 3852
    SubjectKeyIdentifier, Digest, IssuerAndSerialNumber FROM CryptographicMessageSyntax2004 { iso(1) member-body(2) us(840) rsadsi(113549) pkcs(1) pkcs-9(9) smime(16) modules(0) cms-2004(24) };

DeviationListVersion ::= INTEGER {v0(0)}

DeviationList ::= SEQUENCE {
    version DeviationListVersion,
    digestAlgorithm AlgorithmIdentifier OPTIONAL,
    deviations SET OF Deviation
}

Deviation ::= SEQUENCE {
    documents DeviationDocuments,
    descriptions SET OF DeviationDescription
}

DeviationDescription ::= SEQUENCE {
    description PrintableString OPTIONAL,
    deviationType OBJECT IDENTIFIER,
    parameters [0] ANY DEFINED BY deviationType OPTIONAL,
    nationalUse [1] ANY OPTIONAL
}

DeviationDocuments ::= SEQUENCE {
    documentType [0] PrintableString (SIZE(2)) OPTIONAL,
    dscIdentifier DocumentSignerIdentifier OPTIONAL,
    issuingDate [4] IssuancePeriod OPTIONAL,
    documentNumbers [5] SET OF PrintableString OPTIONAL
}

DocumentSignerIdentifier ::= CHOICE {
    issuerAndSerialNumber [1] IssuerAndSerialNumber,
    subjectKeyIdentifier [2] SubjectKeyIdentifier,
    certificateDigest [3] Digest
}

IssuancePeriod ::= SEQUENCE {
    firstIssued GeneralizedTime,
    lastIssued GeneralizedTime
}

CertField ::= CHOICE {
    body CertificateBodyField,
    extension OBJECT IDENTIFIER
}

CertificateBodyField ::= INTEGER {
    generic(0), version(1), serialNumber(2), signature(3), issuer(4), validity(5), subject(6), subjectPublicKeyInfo(7), issuerUniqueID(8), subjectUniqueID(9)
}

Datagroup ::= INTEGER {
    dg1(1), dg2(2), dg3(3), dg4(4), dg5(5), dg6(6), dg7(7), dg8(8), dg9(9), dg10(10), dg11(11), dg12(12), dg13(13), dg14(14), dg15(15), dg16(16), sod(20), com(21)
}

MRZField ::= INTEGER {
    generic(0), documentCode(1), issuingState(2), personName(3), documentNumber(4), nationality(5), dateOfBirth(6), sex(7), dateOfExpiry(8), optionalData(9)
}

-- Base Object Identifiers
id-icao OBJECT IDENTIFIER ::= {2 23 136}
id-icao-mrtd OBJECT IDENTIFIER ::= {id-icao 1}
id-icao-mrtd-security OBJECT IDENTIFIER ::= {id-icao-mrtd 1}
id-icao-DeviationList OBJECT IDENTIFIER ::= {id-icao-mrtd-security 7}
id-icao-DeviationListSigningKey OBJECT IDENTIFIER ::= {id-icao-mrtd-security 8}

-- Deviation Object Identifiers and Parameter Definitions
id-Deviation-CertOrKey OBJECT IDENTIFIER ::= {id-icao-DeviationList 1}
id-Deviation-CertOrKey-DSSignature OBJECT IDENTIFIER ::= {id-Deviation-CertOrKey 1}
id-Deviation-CertOrKey-DSEncoding OBJECT IDENTIFIER ::= {id-Deviation-CertOrKey 2}
id-Deviation-CertOrKey-CSCAEncoding OBJECT IDENTIFIER ::= {id-Deviation-CertOrKey 3}
id-Deviation-CertOrKey-AAKeyCompromised OBJECT IDENTIFIER ::= {id-Deviation-CertOrKey 4}
id-Deviation-LDS OBJECT IDENTIFIER ::= {id-icao-DeviationList 2}
id-Deviation-LDS-DGMalformed OBJECT IDENTIFIER ::= {id-Deviation-LDS 1}
id-Deviation-LDS-DGHashWrong OBJECT IDENTIFIER ::= {id-Deviation-LDS 2}
id-Deviation-LDS-SODSignatureWrong OBJECT IDENTIFIER ::= {id-Deviation-LDS 3}
id-Deviation-LDS-COMInconsistent OBJECT IDENTIFIER ::= {id-Deviation-LDS 4}
id-Deviation-MRZ OBJECT IDENTIFIER ::= {id-icao-DeviationList 3}
id-Deviation-MRZ-WrongData OBJECT IDENTIFIER ::= {id-Deviation-MRZ 1}
id-Deviation-MRZ-WrongCheckDigit OBJECT IDENTIFIER ::= {id-Deviation-MRZ 2}
id-Deviation-Chip OBJECT IDENTIFIER ::= {id-icao-DeviationList 4}
id-Deviation-NationalUse OBJECT IDENTIFIER ::= {id-icao-DeviationList 5}

END
```

---

## 11. REFERENCES (NORMATIVE)

| Reference | Description |
| :--- | :--- |
| FIPS 180-2 | Federal Information Processing Standards Publication (FIPS PUB) 180-2, Secure Hash Standard, August 2002. |
| FIPS 186-4 | Federal Information Processing Standards Publication (FIPS PUB) 186-4, Digital Signature Standard (DSS), July 2013. |
| ISO 3166-1 | ISO/IEC 3166-1: 2006, Codes for the representation of names of countries and their subdivisions — Part 1: Country Codes. |
| ISO/IEC 15946 | ISO/IEC 15946: 2002, Information technology — Security techniques — Cryptographic techniques based on elliptic curves. |
| RFC 3280 | Internet X.509 Public Key Infrastructure Certificate and Certificate Revocation List (CRL) Profile, April 2002. |
| RFC 4055 | Additional Algorithms and Identifiers for RSA Cryptography for use in the Internet X.509 Public Key Infrastructure Certificate and Certificate Revocation List (CRL) Profile, June 2005. |
| RFC 5652 | Cryptographic Message Syntax, September 2009. |
| RFC 5280 | Internet X.509 Public Key Infrastructure Certificate and Certificate Revocation List (CRL) Profile, May 2008. |
| TR 03111 | BSI TR-03111: Elliptic Curve Cryptography v2.0, 2012. |
| X9.62 | Public Key Cryptography For The Financial Services Industry: The Elliptic Curve Digital Signature Algorithm (ECDSA), 7 January 1999. |
| X.509 | ITU-T X.509 \| ISO/IEC 9594-8, 2008: Information technology – Open Systems Interconnection – The Directory: Public-key and attribute certificate frameworks. |
| X.690 | ITU-T X.690 2008: Information technology- ASN.1 encoding rules: Specification of Basic Encoding Rules (BER), Canonical Encoding Rules (CER) and Distinguished Encoding Rules (DER). |
| RFC-RSA | RFC 3447, Public-key cryptography standards (PKCS)#1: RSA cryptography specifications version 2.1, 2003. |
| PKCS#1 | RSA Laboratories Technical Note, PKCS#1 v2.2: RSA cryptography standard, 2012. |
| TLSAES | Advanced Encryption Standard (AES) Ciphersuites for Transport Layer Security (TLS), RFC 3268, June 2002. |
| TLSECC | Elliptic Curve Cryptography (ECC) Cipher Suites for Transport Layer Security (TLS), RFC 4492, May 2006. |
| TLS1.2 | The Transport Layer Security (TLS) Protocol Version 1.2, RFC 5246, August 2008. |
| TLSEXT | Transport Layer Security (TLS) Extensions, RFC 4366, April 2006. |
| SOAP | SOAP Version 1.2 Part 1: Messaging Framework (Second Edition), W3C Recommendation 27 April 2007. |
| HTTPS | HTTP Over TLS, RFC 2818, May 2000. |
| WSI-BP | WS-I Basic Profile available at http://www.ws-i.org/Profiles/BasicProfile-1.1.html |
| WSI-SSBP | WS-I Basic Binding available at http://www.ws-i.org/Profiles/SimpleSoapBindingProfile-1.0.html |

---

## APPENDICES (INFORMATIVE)

### APPENDIX A — LIFETIMES
The following examples illustrate calculation of private key usage periods and public key certificate validity for various scenarios as described in Section 4.

**Example 1**: eMRTDs valid for five years. Document Signer private key usage period: 1 month. Document Signer certificate validity: 5 years + 1 month. CSCA private key usage period: 3 years. CSCA certificate validity: 8 years + 1 month.
**Example 2**: eMRTDs valid for ten years. Document Signer private key usage period: 2 months. Document Signer certificate validity: 10 years + 2 months. CSCA private key usage period: 4 years. CSCA certificate validity: 14 years + 2 months.
**Example 3**: eMRTDs valid for ten years. Document Signer private key usage period: 3 months. Document Signer certificate validity: 10 years + 3 months. CSCA private key usage period: 5 years. CSCA certificate validity: 15 years + 3 months.

### APPENDIX B — CERTIFICATE AND CRL PROFILE REFERENCE TEXT
This appendix replicates brief excerpts of relevant sections from source documents (e.g., RFC 5280, X.690) to assist the reader in understanding the background for some of the requirements specified in the eMRTD certificate and CRL profiles. (See full reference tables in the source document for detailed excerpts on TBSCertificate, signatureAlgorithm, validity, SubjectKeyIdentifier, KeyUsage, BasicConstraints, etc.)

### APPENDIX C — EARLIER CERTIFICATE PROFILES
The certificate profiles in this appendix were specified in the Sixth Edition of ICAO Doc 9303. Although CSCAs MUST issue certificates that comply with the current profiles as specified in Section 7, the earlier profiles are included here for information only as certificates that were issued in compliance with the earlier profiles will be in circulation, and processed by Inspection Systems for several years.

### APPENDIX D — RFC 5280 VALIDATION COMPATIBILITY
This appendix provides guidance to receiving States wishing to use systems that implement the [RFC 5280] certification path and CRL validation algorithms.
- **D.1 Steps Relevant to eMRTD**: Identifies the subset of steps from the [RFC 5280] definition that are required for the eMRTD application and provides the necessary inputs and initialization values.
- **D.2 Steps Not Required by eMRTD**: Covers the remaining steps from the [RFC 5280] definition that are not relevant to the eMRTD application (e.g., policy mapping, name constraints).
- **D.3 Modifications required to process CRLs**: Provides guidance to support the extension of [RFC 5280] based CRL processing to cover revocation checking after a CSCA has undergone a name change.

### APPENDIX E — LDS2 EXAMPLE
The following example illustrates the interactions between the different components of the LDS2 Signature PKI and the LDS2 Authorization PKI.

**Scenario**: The country of Dystopia wants to write travel stamps to passports of citizens of the country of Utopia. Later, the country of Atlantis wants to read travel stamps written by Dystopia on Utopia’s passports.

**Preliminaries**:
- Utopia has installed an LDS2 Travel Stamp application on their passports.
- Both Dystopia and Utopia have set up their LDS2 Authorization PKI.
- Dystopia has set up their LDS1 Signing PKI to issue LDS2 Signer Certificates.
- CVCA certificates and SPOC client and server certificates were exchanged in a trusted manner between Utopia and Dystopia, and between Utopia and Atlantis.
- CSCA certificates have been exchanged in a trusted manner between Dystopia and Atlantis.

**Recurring process to enable Dystopia to electronically stamp Utopia’s eMRTDs**:
- Dystopia requests a DV certificate from Utopia.
- Dystopia’s SPOC uses its SPOC client certificate and Utopia’s SPOC server certificate to initiate a SPOC connection. Then, a request is generated by a Dystopian DV, and sent from SPOC-to-SPOC. Upon request, Utopia generates a foreign DV certificate with read/write access for Dystopia, and the certificate is delivered back via SPOC-to-SPOC.
- Upon receiving the DV certificate from its SPOC, the DV of Dystopia generates Terminal Certificates for the terminals of its borders. Connecting to the passport, the IC on the Utopian passports verifies the terminal certificate of Dystopia with the DV certificate of Dystopia, and the DV certificate of Dystopia with the CVCA certificate of Utopia. The IC then grants read/write access for the Dystopian terminal to the LDS2 Travel Stamp application.

**The process to electronically stamp an eMRTD is as follows**:
- Dystopia creates an electronic travel stamp, and signs it with the private key corresponding to the public key stored in an LDS2 (Travel Stamp) Signer certificate of the LDS2 Signing PKI of Dystopia. The LDS2 Signer certificate is stored on the contactless IC of the Utopian passport.

**Upon encountering the Utopian passport at the border of Atlantis**:
- If reading travel stamps from Utopian passports requires a terminal certificate with read access, a certificate request from Atlantis is sent via SPOC-to-SPOC to Utopia. Upon request, Utopia generates a foreign DV certificate with read-access for Atlantis and sends this certificate to Atlantis via SPOC-to-SPOC. Using that DV certificate, Atlantis generates terminal certificates with read-access for Utopian passports for Atlantis’ terminals. If travel stamps in Utopian passports can be read by any terminal, this step can be omitted.
- To verify a travel stamp of the passport written by Dystopia, Atlantis uses the LDS1 signing PKI of Dystopia: The Dystopian LDS2 Signer certificate stored in the passport is used to verify the travel stamp. Then, the chain is built up, i.e., the Dystopia LDS2 Signer certificate is verified with the Dystopia CSCA certificate received preliminarily.

---
*— END —*
