# Coding standards

The judgement calls that make code read like alba. Cite a rule's heading in a review finding.

## Comments

- Keep a comment only when it carries a why, a unit, or a gotcha the code cannot say.
- Write a doc comment that adds to the name. Let the name and signature carry the rest.
- Leave examples, ADR numbers, and design labels out of comments. The code and the ADRs carry them.

## Names

- Spell out a name that holds a domain concept, in the glossary's full term: `size_tier`, not `tier`. Short names fit closure parameters and locals that live a few lines.
- Give each word one meaning across Rust, TOML, and JSON. When two concepts share a word, rename one.
- Name types and modules for what they hold, in domain words. A field reads as a property of its owner.

## Shape

- Parse input once at the boundary into a type, and trust that type inward.
- Keep validation and logic pure, and apart from I/O. Validate first, then write.
- Give each function one job. An orchestrator calls one-job helpers.
- Make a platform-specific step a named function.
- Reach for a derive or a maintained crate, such as `serde_repr`, before a hand-written trait impl or macro. Ask before adding the crate.
