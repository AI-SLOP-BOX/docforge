---
trigger: always_on
description: Apple / Adobe Pro-tier design system rules to eliminate generic AI aesthetic tells
---

# Professional Frontend Design Guidelines (Anti-AI Aesthetics)

Follow the combined philosophy of **Anthropic Frontend Design**, **UI/UX Pro Max**, **Apple HIG**, and **shadcn semantic systems**:

## 1. Eliminate Generic AI Aesthetics ("AI臭"の徹底排除)
- **NO Generic Warm Cream Backgrounds** (#F4F1EA / beige cards).
- **NO Purple-Cyan Glows Everywhere** as a lazy substitute for intentional contrast.
- **NO Monotonous Card Layouts** where every card has the same border-radius, same padding, and same slide-up animation.
- **NO Random Text Accent Tells**: Never accent a single word in a headline with italic/rainbow/different color without editorial reason.
- **NO Unnecessary Uppercase Eyebrows**: Do not clutter panels with artificial, meaningless UPPERCASE mini-labels above every single control.
- **ZERO Emojis in Desktop/Web UI**: Always use crisp, semantic vector SVG icons. Never use emojis (📄, ⚡, 📷, ⬛, etc.) as system icons.

## 2. Intentional Typography & Layout
- **Font Stack**: Use native system fonts (`-apple-system, BlinkMacSystemFont, 'SF Pro Display', 'SF Pro Text'`) with deliberate weights (400 regular, 500 medium, 600 semibold, 700 bold).
- **Hierarchy by Spacing & Proximity**: Group related controls inside cohesive semantic panels (`.inspector-card`). Let negative space delineate sections instead of dozens of horizontal rules.
- **Precision Data Presentation**: In desktop tools (like PDF, Figma, Lightroom), numbers must feel tactile. Provide clear units (`pt`, `px`, `%`) and use monospace tabular numerals for numerical coordinates and zoom levels.

## 3. Tactile Interaction & Micro-Motion
- **Intentional Feedback**: Interactive elements must acknowledge mouse interaction with subtle state shifts (scale 0.98 on press, subtle background contrast shift on hover).
- **Keycap Badges**: Expose keyboard shortcuts using native keycap styling (`<kbd>⌘S</kbd>`) so power users feel the software is designed for professionals.
- **Floating HUD Design**: Place high-frequency viewport controls (zoom, page jumps, fit-to-width) in a floating glass capsule (`.floating-hud`) with `backdrop-filter: blur(20px)` and subtle rim lighting (`inset 0 1px 0 rgba(255, 255, 255, 0.08)`).

## 4. Accessibility & Robustness
- Ensure interactive target sizes meet minimum usability thresholds (at least 28-32px clickable hit area for compact desktop toolbars, 44px for primary touch targets).
- Maintain WCAG AA compliant contrast ratio (minimum 4.5:1) for all readable text.
- Preserve instant responsiveness: cache expensive operations (like rendered page canvases) to guarantee 0ms latency on repeated visits.
