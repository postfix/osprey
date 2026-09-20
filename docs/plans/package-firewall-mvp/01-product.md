# Product: Package Firewall MVP

Source: `SPEC.md` sections 1, 2, 5–8 (product rules only), 11 (client-facing failures), 13 (completion). Everything technical in that file is deferred to Gates 2 and 3.

## Problem
"My developers and CI jobs install whatever was published a minute ago. When a popular package gets hijacked, we install the bad release hours before any malware feed names it. If I just refuse new releases, installs break even though last week's release would have worked fine. And when a feed finally does name a bad package, nothing stops the next machine from downloading it again."

Who has it: teams installing public JavaScript (npm) and Python (pip) packages on developer machines and CI, who run their own infrastructure and want protection without changing how they install.

## Success metric
In the end-to-end install suite run with real npm and pip clients against harmless test packages: **0 package files delivered that are younger than the waiting period or named on the blocklist**, while **100% of installs that have an older acceptable release still succeed** without anyone editing their dependency requirements or lockfile.

Measured by: the install suite's pass count, plus the firewall's decision log for every delivered and refused file. Speed is a secondary bar, measured and reported rather than promised: a repeat request for something the firewall already knows answers in a few milliseconds.

## Non-goals
- Not proof a package is harmless. Passing the waiting period and the blocklist only means it passed those two checks.
- Does not pick versions. npm and pip still decide what is compatible and still own lockfiles; the firewall only decides what is available.
- Never swaps in a different version or different bytes for an exact request. An exact request for a refused release fails.
- Does not reach packages already installed or already in a client's own cache, nor packages fetched from other indexes, Git or direct URLs, or local files.
- Does not scan package contents, run AI analysis, or collect threat intelligence itself. It reads one blocklist file that someone else produces.
- No screens, no publishing, no login, no search, no audit endpoint, no private packages, no admin API, no multi-server setup.
- One shared policy per installation; no per-team or per-package exemptions, including for popular or locked packages.
- Does not keep serving during an outage of the public registries by handing out stale listings.

## Announcement — the blog post before the feature
Package Firewall is a free, self-hosted gatekeeper that sits between your npm and pip installs and the public registries. It hides any release younger than a waiting period you choose — 24 hours by default — and anything on your malware blocklist, so your existing tools quietly settle on the newest release that has survived the wait. Ask for an exact release that is too new or blocked and you get a clear refusal saying why and when it becomes available, never a silent substitute. Every file is checked against its published fingerprint and the blocklist before a single byte reaches you, and a file that becomes blocked later is refused from then on, even from old links. Update the blocklist file and it takes effect within seconds with no restart; if the blocklist goes missing or stale, the firewall stops handing out packages rather than pretending everything is clean. Point npm and pip at one address and keep working the way you do today.

## Screens
No UI. The operator-visible surface is three commands (run the service, check the settings file, check a blocklist file), the refusal messages clients see, a ready/alive signal, and a decision log.

## Product rules the later gates must keep
| Situation | What the user sees |
| --- | --- |
| Newest release is too young | An older acceptable release is offered instead. |
| Dependency allows a range | The client picks among the releases still offered, under its own original rules. |
| Exact release is too young or blocked | Refused, with a reason; for the waiting period, also when it becomes available. |
| Frozen lockfile names a blocked file | The download is refused; the lockfile is never rewritten. |
| Nothing acceptable is left | The install fails; requirements are never loosened. |
| A release finishes its wait | It becomes available on its own, on the next request. |
| An already-delivered file becomes blocked | Later downloads are refused, including through links handed out earlier. |
| Blocklist missing or past its expiry | All package delivery stops until a valid one is present; an empty but valid blocklist is allowed. |
| A bad replacement blocklist is dropped in | The last good one keeps working and the problem is logged. |
| Python: a new file is added to an old release | The new file serves its own wait; it does not inherit the old release's age. |
| JavaScript: the "latest" label points at a hidden release | "latest" moves to the newest acceptable stable release at or below it; other labels pointing at hidden releases are dropped, not guessed. |
| A release has no publication time | The firewall's own first sighting starts the clock, and a restart does not reset it. |
| Known malware | Always refused, regardless of age, popularity, or lockfiles. |

## Open product questions (resolved 2026-09-17: approved as drafted — name "Package Firewall"; reliability metric is the headline, speed measured and reported)
1. Product name: the specification says "Package Firewall"; the repository is named `osprey`. This document uses "Package Firewall" throughout.
2. The specification's speed targets are treated here as measured-and-reported, not as the success metric. Say so if a speed number should be the headline instead.
