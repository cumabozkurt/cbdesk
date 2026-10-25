<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Preliminary name search for "CBDesk" (risk R-012)

**This is not legal advice and it is not a trademark clearance.** It is a
desk search done with public web search and public registries' APIs on
2026-10-25. It did not query the USPTO, EUIPO TMview, WIPO Global Brand
Database or TÜRKPATENT trademark databases directly (those tools need an
interactive browser session). Absence of a hit here does not mean the name is
free. Before any commercial use, merchandise, store listing or filing, have a
trademark attorney run a full search in Class 9 (software), 42 (SaaS) and 38
(telecommunications) for the US, EU, Türkiye and any other target market.

_Bu belge hukuki tavsiye değildir ve marka tescil/temizleme raporu yerine
geçmez. 2026-10-25 tarihinde herkese açık web aramasıyla yapılmış ön
taramadır. USPTO, EUIPO TMview, WIPO ve TÜRKPATENT veritabanları doğrudan
sorgulanmadı. Ticari kullanımdan, mağaza yayınından veya başvurudan önce bir
marka vekiline tam arama yaptırın._

## Summary

| Area | Result | Risk |
| --- | --- | --- |
| Existing product named "CB Desk" / "CBDesk" | Yes: Coldwell Banker's real-estate intranet "CB Desk" at `mycbdesk.com` (Coldwell Banker Real Estate LLC, Anywhere group) | **Medium–high** for confusion in the abbreviation "CB Desk"; different field (real-estate agent portal vs. remote desktop) |
| Company named cbdesk | `cbdesk GmbH`, Rodalben, Germany (HRB 31864, Amtsgericht Zweibrücken, registered 2018, purpose "E-Commerce") | Medium: identical company name in the EU, unrelated field as far as the register shows |
| Other web hits | A third-party site describing a "CBDesk" team-collaboration product (`mycbdesk.com` profile on amrut.ai) appears to be a generated description of the Coldwell Banker portal and is not reliable | Unclear; ignore until verified |
| Domains | `cbdesk.com` (registered 2016), `cbdesk.org`, `cbdesk.net` (2023) and `mycbdesk.com` (2018) are registered by others. `cbdesk.net` serves a page titled "CB Desk". `.io`, `.app`, `.dev`, `.eu`, `.com.tr` returned no RDAP record, which may only mean the registry has no RDAP service | Domains are not available in the main TLDs |
| GitHub | No user or organization called `cbdesk`, `cb-desk` or `cbdesk-org`. One other repo named `cbDesk` (0 stars, no description). The Packagist package `cbdesk/key-management_api` is an eBay API client under a GitHub name that is no longer resolvable | Low |
| crates.io, npm, PyPI, pub.dev | `cbdesk` and `cb-desk` returned 404 (not taken) on 2026-10-25 | Low; names can be reserved |
| Docker Hub | namespace `cbdesk` has no repositories | Low |
| Apple App Store | Search "cbdesk" returned no app with that name | Low |
| Flathub | `org.cbdesk.CBDesk` was not present. The template id is now `org.cbdesk.CBDesk` and has not been submitted | Low |
| TÜRKPATENT / EUIPO / USPTO mentions | No registration or application for CBDESK was surfaced by web search. This proves nothing about the registers themselves | Unknown |

## Findings in detail

### 1. Coldwell Banker "CB Desk"
`mycbdesk.com` is the login for "CB Desk", described by Coldwell Banker
material as the brand's virtual office and productivity hub for real-estate
agents. "Coldwell Banker" and its logo are trademarks of Coldwell Banker Real
Estate LLC. The company's own documents write the product as "CB Desk" (two
words) and the domain as `mycbdesk.com`. The product is a real-estate
intranet, not remote-access software, so a likelihood-of-confusion claim over
"CBDesk" for remote desktop software is not obvious, but a large brand owner
with an active product of nearly the same name is the main thing to have an
attorney assess. Do not use "CB" in a way that evokes Coldwell Banker (colors,
logo styling, real-estate marketing).

### 2. cbdesk GmbH
A German GmbH with the exact name "cbdesk GmbH" exists (Rodalben,
Rheinland-Pfalz; LEI 391200XHE65V09VZ0C64; managing director listed at
North Data). Its stated purpose is e-commerce. A company name is not a
trademark by itself, but it can support an unregistered-rights or
company-name claim in Germany, and it is a reason to check the German DPMA
register and TMview before publishing in the EU.

### 3. Domains and handles
The project cannot use `cbdesk.com`, `.org`, `.net`. Candidates to verify
with a registrar: `cbdesk.io`, `cbdesk.app`, `cbdesk.dev`, `cbdesk.eu`,
`cbdesk.com.tr`, or a different name entirely. `TRADEMARK.md` asks
contributors not to register domains in the project's name yet; that stays.

### 4. Package names
Reserve nothing automatically. If the name stays, consider publishing a
placeholder under `cbdesk` on crates.io and pub.dev only after the
attorney review, since a registry name is a cheap thing to lose and an
expensive thing to hand back.

## Search terms used
"CBDesk" software; "CB Desk" trademark OR company OR product remote desktop
OR helpdesk; CBDesk trademark EUIPO OR TÜRKPATENT OR USPTO; "cbdesk.com" OR
"CBDesk" remote desktop OR help desk OR CRM company. Registries queried via
public JSON endpoints: crates.io, npm, PyPI, pub.dev, Docker Hub, iTunes
Search, GitHub search, rdap.org.

## Recommended next steps (owner)
1. Search the same term, plus phonetic variants (SeeBeeDesk, CB-Desk, CBDisk),
   yourself in: TMview (https://www.tmdn.org/tmview/), USPTO trademark search
   (https://tmsearch.uspto.gov/), WIPO Global Brand Database
   (https://branddb.wipo.int/), TÜRKPATENT (https://www.turkpatent.gov.tr/).
2. Ask a trademark attorney (Turkey and the EU at least) for a clearance
   opinion that covers Coldwell Banker "CB Desk" and cbdesk GmbH.
3. Decide: keep the name and accept the risk, adopt a modifier
   (e.g. "CBDesk Open"), or rename. A rename is cheapest now: no release,
   store listing or domain exists. `tools/` already contains a rebrand scan
   that helps.
4. If keeping the name, file a Turkish and/or EUIPO application only after
   step 2.
5. Update `TRADEMARK.md` and risk R-012 in the plan once a decision is made.
