# Sponsor Orbit

Orbit is a Rust-native terminal system dashboard — CPU, memory, disk, network, GPU,
process management, plus an AI diagnostics mode. It ships as a ~3 MB single binary and
runs on Linux, macOS, and Windows.

```bash
npm install -g @ziuus/orbit && orbit
```

MIT licensed. Two human maintainers.

---

## Reach

All figures below were pulled from the GitHub API on **2026-10-05**. Anything marked
"pending" is instrumented but was not publishing numbers at the time of writing — ask and
I'll walk you through the live dashboard instead of reading you a stale screenshot.

| Metric | Value |
|---|---|
| GitHub stars | 44 |
| GitHub forks | 3 |
| Commits on `main` | 402 |
| Tagged releases | 30 (latest `v0.11.8`, same day) |
| First commit | 2026-05-29 (rebranded Vanta → Orbit in `v0.11.0`) |
| Human contributors | 2 |
| Published community extensions | 27, all in the public registry |
| Package | `@ziuus/orbit` (npm), `cargo install --git` |
| Install counts | pending — see note below |
| Active installs (unique machines, 24h/7d) | pending — see note below |

**On the two pending rows, honestly:** Orbit runs anonymous telemetry keyed on a hashed
machine ID, counting unique machines rather than sessions. The numbers exist and I can
show you the live dashboard on a call. They are not in this document because I would
rather show you the real thing than have you trust a figure I typed from memory. If a
published number matters to your process, that is the first thing we set up.

---

## Inventory

### 1. In-UI sponsor slot — exclusive

A permanent two-line slot anchored to the bottom of the TUI, rendered on every page:

```
 Sponsored by <your product>
 <one line of copy>
```

Mechanically:

- **Zero integration cost.** The binary polls a JSON endpoint over HTTP, hourly, on a
  background thread only when the slot is on screen. No SDK, no build change, no
  dependency on your side, and no I/O inside the render loop.
- **Live-swappable.** You give me a JSON object; I change one file. Takes effect on the
  next poll. No release, no app store review, no waiting on me to ship.
- **One line + one URL.** Terminal real estate is genuinely scarce. I will not sell this
  as 60 characters and then load it with tracking pixels.
- **Opt-out exists, and I'll tell you that up front.** Nothing about a sponsored line
  should be hidden from users, so the slot is documented in the README. Community
  backlash is a real cost to a developer tool's reputation, and I will not take that risk
  on your behalf.

### 2. README sponsor section — exclusive

Logo + one line in the README, alongside the current Vercel and Railway credits. GitHub
renders the logo on the repo front page.

### 3. GitHub Sponsors

One-off or recurring, via [.github/FUNDING.yml](.github/FUNDING.yml). Goes to the
project generally rather than to placement.

---

## Pricing

**$3,000/month** for the in-UI slot, exclusive, month-to-month.

That's the number, stated once, no "starting at." Here's the honest math on why: at $36k
annualized, you're roughly at the going rate for a dev-adjacent property with five-figure
annual traffic — and I do not have five-figure annual traffic yet. This price is
defensible *because* of what terminal inventory is (no ad blockers, no scroll, an audience
that buys infrastructure), not because of my current numbers.

So, two ways to make it work:

- **Prove it first.** Take the slot for 60 days free. If the numbers I show you on day
  one aren't worth it, walk away with no hard feelings and I won't chase you again. After
  the trial it's $3,000/month.
- **Start smaller.** $600/month gets you the slot today and a real customer relationship
  in your account. If the reporting holds up, we ratchet up.

Either way the term is month-to-month. I don't want you locked into an annual contract
against numbers neither of us had when you signed.

---

## What I won't do

- Sell the same slot twice, or run a bidding war for it.
- Track users, fingerprint machines, or add analytics to the sponsor line. The slot is
  text and a URL. Nothing else.
- Claim a sponsor who isn't paying. Every logo in this repo reflects a real arrangement.
- Put a sponsor in front of users who can't turn it off without editing a config file.

## Contact

**Noel Paul Tomy** — creator and maintainer
sponsor@orbit-tui.com · https://github.com/ziuus

## Past sponsors

| Company | Placement | Period |
|---|---|---|
| Vercel | In-UI slot, README | 2026 |
| Railway | README | 2026 |