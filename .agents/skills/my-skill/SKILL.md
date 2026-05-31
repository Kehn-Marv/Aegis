
---
name: aegis-ui-design
description: ui design inspiration for aegis. Use this when you need to design a ui for aegis.
---

# UI Design System for Aegis Inspired by Studio Console SC-48

## 1. Visual Theme & Atmosphere

This design system embodies authentic skeuomorphism through a premium studio mixing console aesthetic. The visual language combines warm, earthy tones with deep blacks and amber accents to evoke the tactile, analog experience of professional audio equipment. Every element—from beveled buttons to inset shadows—creates the illusion of physical depth and weight. The design prioritizes texture and realism: brushed aluminum surfaces, cream-colored chassis, warm LCD displays, and metallic knobs all work together to transport users into a vintage-meets-digital hybrid space. It's a celebration of hands-on craftsmanship in a digital medium, where interaction feels visceral and satisfying.

**Key Characteristics**

- Deeply tactile, skeuomorphic interface mimicking professional studio hardware
- Warm earth-tone palette (beige, tan, dark brown) with bright amber accents
- Rich, multi-layered shadows creating strong depth and beveled edges
- Amber LCD displays and neon-green status indicators for authentic hardware feel
- Retro-futuristic blend: analog warmth meets digital precision
- High contrast between UI elements and backgrounds for clarity
- Emphasis on realistic textures and material weight

## 2. Color Palette & Roles

### Primary

- **Studio Tan** (`#6A6245`): Dominant background and surface color; appears 394 times throughout the interface. Used for cards, containers, and main structural elements.
- **Deep Chocolate** (`#504428`): Secondary surface tone and depth layer; used 84 times for subtle contrast and internal card backgrounds.

### Accent Colors

- **Warm Amber** (`#E87C14`): Primary call-to-action and highlight color; applied to buttons, icons, and text on dark displays. Conveys warmth and energy—the visual heart of the console.
- **Neon Green** (`#44C464`): Secondary status indicator and success state; used for live indicators and positive confirmations.
- **Cream** (`#FFFADA`): High-contrast text and foreground elements on dark backgrounds; appears 6 times for maximum legibility.

### Interactive

- **Medium Brown** (`#100F08`): Interactive element borders and subtle outlines; provides depth without overwhelming.
- **Nearly Black** (`#0A0A06`): Deep shadow and ultra-dark backgrounds; creates maximum contrast for LCD display areas.
- **Pure Black** (`#000000`): Authentic black display backgrounds and text on light surfaces; used 7 times for stark contrast.

### Neutral Scale

- **Charcoal** (`#1A1A0E`): Dark foreground and text on lighter backgrounds; used 7 times.

### Surface & Borders

- **Warm Gold** (`#D4C020`): Warning state indicator; used 5 times for cautionary UI elements.
- **Amber Secondary** (`#F5A840`): Alternative warm accent for secondary warnings and soft alerts.

### Semantic / Status

- **Error Red** (`#D43020`): Error and danger state indicator; used 3 times for critical alerts.
- **Success Green** (`#22C45A`): Confirmation and positive feedback; reinforces successful actions.

## 3. Typography Rules

### Font Family

**Primary Font:** Inter  
Fallback stack: `Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif`

**Secondary Font:** Inter (mono for displays and technical text when needed)  
Fallback stack: `'Monaco', 'Courier New', monospace`

### Hierarchy

| Role | Font | Size | Weight | Line Height | Letter Spacing | Notes |
|------|------|------|--------|-------------|-----------------|-------|
| Display / Hero | Inter | 30px | 700 | 36px | 0px | Large headings; section titles like "MASTER CONTROL" |
| Heading 1 | Inter | 20px | 900 | 28px | 0px | Primary section headers; bold and prominent |
| Heading 4 | Inter | 14px | 700 | 20px | 0px | Card titles and subsection headers |
| Body Text | Inter | 14px | 400 | 20px | 0px | Standard body copy and descriptions |
| Button Text | Inter | 16px | 400 | 24px | 0px | Large primary button labels |
| Small Button | Inter | 10px | 900 | 15px | 0px | Navigation and secondary button text |
| Label / Caption | Inter | 14px | 900 | 20px | 0px | Bold labels and inline text; control labels |
| Link | Inter | 10px | 700 | 15px | 0px | Navigation links and secondary actions |
| Code / Technical | Inter | 9px | 700 | 13.5px | 0px | Monospace-style info; small UI labels |

### Principles

- **Contrast-Driven:** All type uses high-contrast color pairs (light text on dark, dark text on light) for legibility.
- **Weight Emphasis:** Bold weights (700, 900) signal interactive and important elements; 400-weight for secondary information.
- **Scale Consistency:** Sizes follow a logical progression; each level is distinctly different from neighbors.
- **Hierarchy Through Size & Weight:** Combine size and weight to create clear visual hierarchy without overreliance on color alone.
- **Readability First:** All body text is 14px or larger; links and captions scale down but remain readable.

## 4. Component Stylings

### Buttons

#### Primary Action Button (Large)
- **Background:** `rgba(0, 0, 0, 0)` (transparent with shadow illusion)
- **Text Color:** `#FFFADA` (cream)
- **Font Size:** `10px`
- **Font Weight:** `900`
- **Padding:** `8px 24px`
- **Border Radius:** `4px`
- **Border:** `0px` (none)
- **Box Shadow:** `rgba(255, 255, 255, 0.65) 0px 2px 5px 0px inset, rgba(0, 0, 0, 0.45) 0px -3px 7px 0px inset, rgba(0, 0, 0, 0.45) 0px 6px 14px 0px, rgba(0, 0, 0, 0.25) 0px 2px 4px 0px, rgb(107, 48, 0) 0px 0px 0px 1px`
- **Height:** `31px`
- **Line Height:** `15px`
- **Hover State:** Increase inner shadow opacity by 10%; darken outer shadow slightly.

#### Call-to-Action Button (CTA)
- **Background:** `rgba(0, 0, 0, 0)` (transparent)
- **Text Color:** `#6A6245` (studio tan)
- **Font Size:** `16px`
- **Font Weight:** `400`
- **Padding:** `12px 0px`
- **Border Radius:** `8px`
- **Border:** `0px` (none)
- **Box Shadow:** `rgba(255, 255, 255, 0.55) 0px 2px 4px 0px inset, rgba(0, 0, 0, 0.28) 0px -2px 5px 0px inset, rgba(0, 0, 0, 0.38) 0px 5px 10px 0px, rgba(80, 70, 40, 0.5) 0px 0px 0px 1px`
- **Height:** `48px`
- **Line Height:** `24px`
- **Width:** `268px` (default; responsive)
- **Hover State:** Darken text to `#504428`; increase shadow depth by 15%.

#### Secondary Button (Rounded Knob Style)
- **Background:** `rgba(0, 0, 0, 0)` (transparent)
- **Text Color:** `#6A6245` (studio tan)
- **Font Size:** `16px`
- **Font Weight:** `400`
- **Padding:** `0px` (circular)
- **Border Radius:** `9999px` (fully rounded)
- **Border:** `0px` (none)
- **Box Shadow:** `rgba(255, 255, 255, 0.65) 0px 2px 5px 0px inset, rgba(0, 0, 0, 0.45) 0px -3px 7px 0px inset, rgba(0, 0, 0, 0.45) 0px 6px 14px 0px, rgba(0, 0, 0, 0.25) 0px 2px 4px 0px, rgb(107, 48, 0) 0px 0px 0px 1px`
- **Height:** `80px`
- **Width:** `80px`
- **Line Height:** `24px`
- **Hover State:** Brighten text color; deepen inset highlight shadow.

#### Tertiary Button (Small Control)
- **Background:** `rgba(0, 0, 0, 0)` (transparent)
- **Text Color:** `rgba(80, 68, 40, 0.7)` (muted brown)
- **Font Size:** `9px`
- **Font Weight:** `700`
- **Padding:** `8px 16px`
- **Border Radius:** `4px`
- **Border:** `0px` (none)
- **Box Shadow:** `rgba(255, 255, 255, 0.55) 0px 2px 4px 0px inset, rgba(0, 0, 0, 0.28) 0px -2px 5px 0px inset, rgba(0, 0, 0, 0.38) 0px 5px 10px 0px, rgba(80, 70, 40, 0.5) 0px 0px 0px 1px`
- **Height:** `29.5px`
- **Line Height:** `13.5px`
- **Hover State:** Increase text opacity to full; add subtle highlight.

### Cards & Containers

#### Main Console Card
- **Background:** `#6A6245` (studio tan)
- **Border Radius:** `8px`
- **Border:** `1px solid #504428` (deep chocolate outline)
- **Box Shadow:** `rgba(255, 255, 255, 0.55) 0px 3px 6px 0px inset, rgba(0, 0, 0, 0.3) 0px -4px 10px 0px inset, rgba(80, 70, 40, 0.55) 0px 18px 55px 0px, rgba(0, 0, 0, 0.3) 0px 4px 12px 0px, rgba(90, 80, 50, 0.5) 0px 0px 0px 1px`
- **Padding:** `24px`
- **Gap Between Items:** `24px`

#### Display Panel (LCD Style)
- **Background:** `#1A1A0E` (charcoal/near-black)
- **Border Radius:** `4px`
- **Border:** `1px solid #0A0A06` (nearly black)
- **Box Shadow:** `rgba(0, 0, 0, 0.45) 0px 6px 14px 0px inset`
- **Padding:** `16px`
- **Text Color:** `#E87C14` (warm amber) for active text; `#504428` (muted) for secondary

#### Secondary Card (Module)
- **Background:** `#504428` (deep chocolate)
- **Border Radius:** `6px`
- **Border:** `1px solid #100F08` (medium brown)
- **Box Shadow:** `rgba(255, 255, 255, 0.45) 0px 1px 3px 0px inset, rgba(0, 0, 0, 0.3) 0px 3px 8px 0px`
- **Padding:** `16px`
- **Margin:** `12px`

### Inputs & Forms

#### Text Input Field
- **Background:** `#0A0A06` (nearly black) with `rgba(0, 0, 0, 0.28)` inset shadow
- **Border Radius:** `4px`
- **Border:** `1px solid #100F08` (medium brown)
- **Box Shadow:** `rgba(255, 255, 255, 0.45) 0px 1px 2px 0px inset`
- **Padding:** `12px`
- **Text Color:** `#FFFADA` (cream) for input; `#6A6245` (studio tan) for placeholder
- **Font Size:** `14px`
- **Line Height:** `20px`
- **Focus State:** Border color to `#E87C14` (warm amber); increase inset shadow opacity to 0.6.

#### Checkbox / Toggle
- **Size:** `20px × 20px`
- **Background:** `#504428` (deep chocolate)
- **Border Radius:** `3px`
- **Border:** `1px solid #100F08` (medium brown)
- **Box Shadow:** `rgba(255, 255, 255, 0.45) 0px 1px 3px 0px inset`
- **Checked State:** Inner background `#E87C14` (warm amber) with checkmark in `#FFFADA` (cream)

#### Slider / Range Input
- **Track Background:** `#504428` (deep chocolate)
- **Track Height:** `6px`
- **Thumb Size:** `20px × 20px`
- **Thumb Background:** `#6A6245` (studio tan)
- **Thumb Border Radius:** `50%` (circular)
- **Thumb Shadow:** `rgba(0, 0, 0, 0.3) 0px 2px 4px 0px`
- **Active Track Color:** `#E87C14` (warm amber) from start to thumb position

### Navigation

#### Navigation Bar
- **Background:** `#6A6245` (studio tan)
- **Height:** `60px`
- **Box Shadow:** `rgba(255, 255, 255, 0.55) 0px 3px 6px 0px inset, rgba(0, 0, 0, 0.3) 0px -4px 10px 0px inset, rgba(80, 70, 40, 0.55) 0px 18px 55px 0px, rgba(0, 0, 0, 0.3) 0px 4px 12px 0px`
- **Padding:** `0px 20px`
- **Border Radius:** `8px`

#### Navigation Link
- **Text Color:** `#6A6245` (studio tan) at rest; `#E87C14` (warm amber) on hover
- **Font Size:** `14px`
- **Font Weight:** `400`
- **Padding:** `0px 16px`
- **Line Height:** `24px`
- **Border Bottom on Active:** `3px solid #E87C14` (warm amber)
- **Transition:** `color 200ms ease, border-color 200ms ease`

#### Status Indicator Badges
- **Success Badge:** Background `#22C45A`, text `#FFFADA`, `8px 12px` padding, `20px` border-radius
- **Warning Badge:** Background `#D4C020`, text `#1A1A0E`, `8px 12px` padding, `4px` border-radius
- **Error Badge:** Background `#D43020`, text `#FFFADA`, `8px 12px` padding, `4px` border-radius
- **Info Badge:** Background `#504428`, text `#FFFADA`, `8px 12px` padding, `4px` border-radius

## 5. Layout Principles

### Spacing System

**Base Unit:** `4px`

**Scale:**
- `4px`: Minimal micro-spacing within components
- `8px`: Padding inside small controls; gaps between tiny elements
- `12px`: Standard padding for input fields and small cards
- `16px`: Padding for medium-sized cards and containers; standard button padding
- `20px`: Padding for large sections; common margin between text blocks
- `24px`: Gap between major card sections; standard container padding
- `28px`: Margin between distinct layout sections
- `32px`: Gap between major layout blocks
- `40px`: Padding for extra-large containers; spacing before/after hero sections
- `48px`: Padding for full-width sections; major layout spacing
- `64px`: Gap between separate page sections; maximum standard spacing

**Usage Context:**
- Micro-interactions and internal component spacing: `4px`, `8px`
- Standard padding and margins: `16px`, `20px`, `24px`
- Major section separation: `32px`, `40px`, `48px`, `64px`

### Grid & Container

- **Max Width:** `1400px` for main container
- **Column Strategy:** 12-column grid; components span 1–12 columns depending on breakpoint and context
- **Gutters:** `24px` between columns at desktop; `16px` at tablet; `12px` at mobile
- **Padding Edges:** `20px` on desktop; `16px` on tablet; `12px` on mobile
- **Section Patterns:**
  - Hero/Header: Full-width with `48px` vertical padding
  - Content Cards: 2–3 columns on desktop; 1–2 on tablet; 1 on mobile
  - Sidebar Layouts: 75/25 or 70/30 split; sidebar collapses below `768px`

### Whitespace Philosophy

Whitespace is a core material. Generous breathing room between elements and sections creates visual calm and improves scanability. The design uses whitespace to establish hierarchy and focus user attention. Clustered information uses tighter spacing (`12px`–`16px`); section breaks use broader spacing (`32px`–`64px`). Internal padding within containers always exceeds the gap between container neighbors, creating natural visual grouping.

### Border Radius Scale

- **`0px`:** Inputs requiring geometric precision; true rectangular edges
- **`3px` - `4px`:** Subtle rounding; small buttons, badges, and tight controls
- **`6px` - `8px`:** Standard card corners and medium UI elements
- **`12px` - `16px`:** Large containers and spacious card components
- **`20px`:** Pill-shaped buttons and soft rectangular elements
- **`50% / 9999px`:** Fully circular knobs, badges, and avatar containers

## 6. Depth & Elevation

| Level | Treatment | Use |
|-------|-----------|-----|
| Raised (Level 1) | `rgba(255, 255, 255, 0.65) 0px 2px 5px 0px inset, rgba(0, 0, 0, 0.45) 0px -3px 7px 0px inset, rgba(0, 0, 0, 0.45) 0px 6px 14px 0px, rgba(0, 0, 0, 0.25) 0px 2px 4px 0px, rgb(107, 48, 0) 0px 0px 0px 1px` | Small buttons, knobs, and interactive controls |
| Elevated (Level 2) | `rgba(255, 255, 255, 0.55) 0px 2px 4px 0px inset, rgba(0, 0, 0, 0.28) 0px -2px 5px 0px inset, rgba(0, 0, 0, 0.38) 0px 5px 10px 0px, rgba(80, 70, 40, 0.5) 0px 0px 0px 1px` | Secondary buttons, form fields, and tertiary controls |
| Floating (Level 3) | `rgba(255, 255, 255, 0.55) 0px 3px 6px 0px inset, rgba(0, 0, 0, 0.3) 0px -4px 10px 0px inset, rgba(80, 70, 40, 0.55) 0px 18px 55px 0px, rgba(0, 0, 0, 0.3) 0px 4px 12px 0px, rgba(90, 80, 50, 0.5) 0px 0px 0px 1px` | Main cards, panels, and navigation containers |
| Deep Shadow (Error) | `rgba(220, 48, 48, 0.8) 0px 0px 8px 0px` | Error state indicators and critical alerts |

**Shadow Philosophy:**

Shadows in this design system are multidimensional, combining inset highlights with outer depth shadows to create authentic beveled and embossed effects. This approach mimics real studio hardware where light reflects off curved edges and physical depth is paramount. Each elevation level builds complexity: lower levels have subtle single shadows, higher levels compound inset and outer shadows to create dramatic visual depth. The warm brown border (`rgb(107, 48, 0)`) and taupe undertones (`rgba(80, 70, 40, 0.5)`) ground shadows in the warm, earthy palette. Error states use crisp red glows (`rgba(220, 48, 48, 0.8)`) to signal critical issues without heavy shadow distortion.

## 7. Do's and Don'ts

### Do

- **Use the warm amber (`#E87C14`) sparingly** for true primary actions and active states; its intensity demands restraint.
- **Layer shadows intentionally:** Combine inset and outer shadows to create depth; never use flat shadows alone.
- **Maintain high contrast between text and backgrounds:** Minimum 4.5:1 ratio for all body text.
- **Apply border radius consistently:** Use `4px`–`8px` for most elements; reserve `50%` for circular controls only.
- **Leverage the cream color (`#FFFADA`) for text on dark backgrounds:** It's more readable and warm than pure white.
- **Group related controls with consistent spacing:** Use `16px`–`24px` gaps to create visual families.
- **Respect the padding hierarchy:** Outer padding exceeds internal gaps to maintain clear grouping.
- **Use status colors (`#22C45A` green, `#D43020` red) purposefully** for user feedback; reserve them for semantic meaning.
- **Test interactive elements at actual size:** Buttons and knobs should feel tactile and appropriately sized.

### Don't

- **Mix transparency levels inconsistently:** Button backgrounds should use `rgba(0, 0, 0, 0)` with shadow definition; avoid floating semi-transparent fills.
- **Overuse bold weights (900):** Reserve `font-weight: 900` for labels and small button text; body copy uses `400`.
- **Abandon the inset shadow effect:** The beveled, 3D appearance is core to the skeuomorphic identity; flat design contradicts the system.
- **Apply border radius to text—only to containers:** Text inherits roundness from its parent container only.
- **Use pure white (`#FFFFFF`) or pure black (`#000000`) broadly:** Stick to `#FFFADA` and `#1A1A0E` for warmth and authenticity.
- **Create buttons smaller than `29px` height:** Minimum touch target is `44px`; avoid cramped controls.
- **Combine all four status colors in a single component:** Use one semantic color per element; layering confuses intent.
- **Skip the border color on cards and buttons:** The subtle `1px` border (e.g., `rgb(107, 48, 0)`, `rgba(90, 80, 50)`) grounds the shadow effect.
- **Use color alone to convey meaning:** Pair color with icons, labels, or text for accessibility.
- **Forget to test shadows on light and dark backgrounds:** Shadows adapt per elevation level and context.

## 8. Responsive Behavior

### Breakpoints

| Breakpoint Name | Width | Key Changes |
|-----------------|-------|------------|
| Mobile | `< 480px` | Single column; `12px` padding; `8px` gutters; buttons full-width; font sizes reduce by 1–2px |
| Tablet Small | `480px`–`768px` | 2 columns; `16px` padding; `12px` gutters; button width auto; spacing reduces slightly |
| Tablet Large | `768px`–`1024px` | 3 columns; `20px` padding; `16px` gutters; flexible button widths; standard spacing |
| Desktop | `1024px`–`1400px` | 4 columns; `24px` padding; `24px` gutters; full feature set; maximum spacing |
| Desktop XL | `≥ 1400px` | 6+ columns; `32px` padding; `28px` gutters; enhanced spacing and maximum visual breathing room |

### Touch Targets

- **Minimum interactive size:** `44px × 44px` (button, link, icon)
- **Minimum spacing between touch targets:** `8px` (gap to avoid accidental overlap)
- **Large buttons and controls:** `48px`–`80px` height for primary actions
- **Small controls (toggles, checkboxes):** `20px × 20px` minimum with expanded touch area via padding
- **Navigation items:** `48px` height with `16px` horizontal padding for comfortable tapping

### Collapsing Strategy

- **Navigation:** Horizontal at desktop (`≥ 768px`); collapses to hamburger menu at tablet and mobile with full-screen overlay.
- **Cards and Grids:** 3 columns desktop → 2 columns tablet → 1 column mobile; padding decreases (`24px` → `16px` → `12px`).
- **Modals and Overlays:** Full viewport on mobile (`100% width`); constrained to `90vw` or max `600px` on tablet/desktop.
- **Sidebar Layouts:** Float alongside content at desktop; stacks above at tablet/mobile; sidebar full-width at collapse.
- **Form Layouts:** 2-column at desktop; 1-column on tablet/mobile; input fields grow to full available width below `768px`.
- **Spacing Scale:** All `px` values reduce by proportional factors: `24px` → `16px` → `12px` as viewport shrinks.
- **Font Sizes:** Body text remains `14px` to `16px` for readability; headings reduce by 2–4px on mobile.

## 9. Agent Prompt Guide

### Quick Color Reference

- **Primary CTA:** Warm Amber (`#E87C14`) — use for high-priority buttons and active states
- **Background / Main Surface:** Studio Tan (`#6A6245`) — the dominant color; use for cards, panels, containers
- **Secondary Surface:** Deep Chocolate (`#504428`) — used for nested cards and visual separation
- **Heading Text:** Deep Chocolate (`#504428`) or Studio Tan (`#6A6245`) on light backgrounds; Cream (`#FFFADA`) on dark
- **Body Text:** Studio Tan (`#6A6245`) or Deep Chocolate (`#504428`) on light; Cream (`#FFFADA`) on dark LCD displays
- **Display Background (LCD):** Nearly Black (`#0A0A06`) or Charcoal (`#1A1A0E`) — for authentic screen aesthetic
- **Success State:** Success Green (`#22C45A`)
- **Error State:** Error Red (`#D43020`)
- **Warning State:** Warm Gold (`#D4C020`)
- **Neutral Text:** Muted Brown (`rgba(80, 68, 40, 0.7)`) for disabled or secondary UI
- **Borders & Outlines:** Medium Brown (`#100F08`) or `rgb(107, 48, 0)` depending on elevation context

### Iteration Guide

1. **Always apply multi-layered shadows:** Every interactive component (buttons, cards) must combine inset highlight + outer depth shadow + subtle border color. Flat design violates the skeuomorphic identity.

2. **Maintain color contrast rigorously:** Text on backgrounds must achieve 4.5:1 minimum WCAG AA ratio. Use `#FFFADA` (cream) on dark; `#6A6245` or `#504428` on light.

3. **Size interactive elements properly:** Buttons ≥ `44px` height; knobs ≥ `60px` diameter; form inputs ≥ `40px` height. Touch targets must never be cramped.

4. **Reserve bold typography (900 weight) for labels and small UI:** Body text is always `400` weight; only headings, labels, and small buttons use `700`–`900` weights.

5. **Respect the spacing scale:** Use multiples of `4px` (4, 8, 12, 16, 20, 24, 28, 32, 40, 48, 64). Never use arbitrary spacing like `11px` or `17px`.

6. **Apply warm amber (`#E87C14`) only to primary actions and active states:** It's a high-intensity accent; overuse dilutes its signal value. Restrict to 1–3 elements per screen.

7. **Use semantic status colors deliberately:** Green for success, red for error, gold for warning. Pair with icons and labels—never color alone.

8. **Enforce rounded corners by context:** Standard buttons/cards use `4px`–`8px`; soft UI elements use `12px`–`20px`; knobs/avatars use `50%`. No inconsistent mid-values like `6px` on one button and `3px` on another.

9. **Adapt spacing across breakpoints:** Mobile uses `12px` padding; tablet `16px`; desktop `24px`–`32px`. Recalculate gutters and gaps proportionally for each breakpoint.

10. **Test shadows on multiple backgrounds:** Shadows must remain legible on both light (`#6A6245`) and dark (`#1A1A0E`) surfaces. Adjust outer shadow opacity if needed for contrast.

11. **Prioritize font sizes from the hierarchy table:** Never deviate from specified sizes (20px, 14px, 10px, etc.); keep typography predictable and scannable.

12. **Add borders to all shadowed elements:** Even though shadows define depth, a subtle `1px` border (e.g., `rgb(107, 48, 0)`) completes the beveled effect and grounds the component in the warm palette.