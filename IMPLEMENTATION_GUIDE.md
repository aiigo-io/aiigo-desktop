# Aiigo Desktop - Technical Implementation Guide

## Overview

This guide outlines the technical implementation approach for building the Aiigo Desktop platform, focusing on UI/UX design, component architecture, and development workflow.

---

## UI Design System

### Design Foundation

#### Color Palette

**Primary Colors**
```
Primary Blue:     #1E40AF (for actions, links, highlights)
Primary Dark:     #0F172A (for backgrounds, headers)
Primary Light:    #3B82F6 (for hover states)

Success Green:    #10B981 (for positive values, confirmations)
Warning Orange:   #F59E0B (for alerts, pending states)
Danger Red:       #EF4444 (for losses, errors, deletions)
Neutral Gray:     #64748B (for secondary text, borders)
```

**Background Layers**
```
Level 0 (Base):   #0F172A (darkest - app background)
Level 1 (Cards):  #1E293B (dark - cards, panels)
Level 2 (Hover):  #334155 (medium - hover states)
Level 3 (Active): #475569 (lighter - active states)
```

**Text Colors**
```
Primary Text:     #F8FAFC (white - main content)
Secondary Text:   #CBD5E1 (light gray - descriptions)
Tertiary Text:    #94A3B8 (gray - timestamps, labels)
Disabled Text:    #64748B (dark gray - disabled states)
```

#### Typography

**Font Stack**
```css
--font-primary: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
--font-mono: 'JetBrains Mono', 'Fira Code', 'Courier New', monospace;
```

**Type Scale**
```css
--text-xs:    0.75rem  (12px)  /* Labels, captions */
--text-sm:    0.875rem (14px)  /* Secondary text */
--text-base:  1rem     (16px)  /* Body text */
--text-lg:    1.125rem (18px)  /* Emphasized text */
--text-xl:    1.25rem  (20px)  /* Card headers */
--text-2xl:   1.5rem   (24px)  /* Section headers */
--text-3xl:   1.875rem (30px)  /* Page headers */
--text-4xl:   2.25rem  (36px)  /* Dashboard values */
```

#### Spacing System

```css
--space-1:  0.25rem  (4px)
--space-2:  0.5rem   (8px)
--space-3:  0.75rem  (12px)
--space-4:  1rem     (16px)
--space-5:  1.5rem   (24px)
--space-6:  2rem     (32px)
--space-8:  3rem     (48px)
--space-10: 4rem     (64px)
```

#### Border Radius

```css
--radius-sm:  0.25rem  (4px)   /* Input fields */
--radius-md:  0.5rem   (8px)   /* Buttons, cards */
--radius-lg:  0.75rem  (12px)  /* Large panels */
--radius-xl:  1rem     (16px)  /* Modals */
--radius-full: 9999px          /* Pills, avatars */
```

---

## Application Layout Structure

### Main Layout Architecture

```
┌─────────────────────────────────────────────────────────┐
│ Title Bar (Tauri Custom) - 40px                         │
├──────────┬──────────────────────────────────────────────┤
│          │                                              │
│ Sidebar  │  Main Content Area                           │
│  240px   │                                              │
│          │  ┌────────────────────────────────────────┐ │
│  Nav     │  │ Page Header                            │ │
│  Items   │  │  - Title, Actions, Breadcrumbs         │ │
│          │  └────────────────────────────────────────┘ │
│          │                                              │
│          │  ┌────────────────────────────────────────┐ │
│          │  │                                        │ │
│          │  │  Page Content                          │ │
│          │  │  (Scrollable)                          │ │
│          │  │                                        │ │
│          │  │                                        │ │
│          │  └────────────────────────────────────────┘ │
│          │                                              │
└──────────┴──────────────────────────────────────────────┘
```

### Sidebar Navigation

```
┌──────────────────┐
│  AIIGO           │  Logo & Brand
├──────────────────┤
│                  │
│ 🏠 Dashboard     │  Main Navigation
│ 💼 Portfolio     │  (Icon + Label)
│ 💸 Transactions  │
│ 📊 Markets       │
│ 🔄 Swap          │
│                  │
├──────────────────┤  Divider
│                  │
│ 🚀 VC Platform   │  Secondary Navigation
│ 📁 Projects      │
│ 💰 Investments   │
│                  │
├──────────────────┤  Divider
│                  │
│ ⚙️ Settings      │  Footer Navigation
│ 👤 Profile       │
│                  │
└──────────────────┘
```

**Sidebar States:**
- Default: 240px wide
- Collapsed: 64px wide (icons only)
- Responsive: Hidden on mobile, drawer overlay

---

## Key Screen Designs

### 1. Dashboard Screen

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│ Dashboard                                    [Timeframe] │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ ┌─────────────────────┐  ┌─────────────────────────┐   │
│ │ Total Balance       │  │ 24h Change              │   │
│ │ $125,487.32         │  │ +$4,231.21 (+3.49%)    │   │
│ │ ≈ 3.45 BTC          │  │ ↗                       │   │
│ └─────────────────────┘  └─────────────────────────┘   │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Portfolio Value Chart (7 days)                     │  │
│ │ ╭─╮                                                │  │
│ │ │ ╰─╮   ╭─╮                                        │  │
│ │ ╯   ╰─╮╯ ╰─╮                                       │  │
│ │         ╰───╯                                      │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌──────────────────────┐  ┌────────────────────────┐   │
│ │ Asset Allocation     │  │ Top Movers             │   │
│ │                      │  │ ┌──────────────────┐   │   │
│ │    ◉ BTC 45%        │  │ │ BTC   +5.2%  ↗   │   │   │
│ │    ◉ ETH 30%        │  │ │ ETH   +3.1%  ↗   │   │   │
│ │    ◉ Other 25%      │  │ │ SOL   -2.4%  ↘   │   │   │
│ │                      │  │ └──────────────────┘   │   │
│ └──────────────────────┘  └────────────────────────┘   │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Recent Transactions                    [View All]  │  │
│ │ ┌────────────────────────────────────────────────┐ │  │
│ │ │ ↗ Sent BTC          -0.05 BTC    2 hours ago  │ │  │
│ │ │ ↙ Received ETH      +1.2 ETH     5 hours ago  │ │  │
│ │ └────────────────────────────────────────────────┘ │  │
│ └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Components:**
- `<StatCard>`: Large value display with subtitle
- `<PortfolioChart>`: Line/area chart with timeframe selector
- `<PieChart>`: Asset allocation visualization
- `<AssetMoverCard>`: Compact asset with price change
- `<TransactionListItem>`: Transaction with icon, amount, timestamp

---

### 2. Portfolio Screen

**Layout:**
```
┌─────────────────────────────────────────────────────────┐
│ Portfolio              [Search] [Filter] [Sort] [+Add]  │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Asset        Balance        Value         Change   │  │
│ ├────────────────────────────────────────────────────┤  │
│ │ [BTC] Bitcoin                                      │  │
│ │              2.45 BTC     $89,234.50     +2.3%    │  │
│ │              ▓▓▓▓▓▓▓▓░░░░░░ 45% of portfolio      │  │
│ ├────────────────────────────────────────────────────┤  │
│ │ [ETH] Ethereum                                     │  │
│ │              12.8 ETH     $38,912.00     +1.8%    │  │
│ │              ▓▓▓▓░░░░░░░░░░ 30% of portfolio      │  │
│ ├────────────────────────────────────────────────────┤  │
│ │ [USDT] Tether                                      │  │
│ │              25,000 USDT  $25,000.00     +0.0%    │  │
│ │              ▓▓░░░░░░░░░░░░ 15% of portfolio      │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ [Wallet 1] [Wallet 2] [Exchange]  ← Tabs for sources    │
└─────────────────────────────────────────────────────────┘
```

**Components:**
- `<AssetTable>`: Sortable, filterable table
- `<AssetRow>`: Expandable row with details
- `<ProgressBar>`: Visual allocation percentage
- `<WalletTabs>`: Toggle between different sources

**Interactions:**
- Click row to expand and show wallet addresses, transactions
- Click asset icon to view asset detail page
- Hover to show quick actions (send, receive, swap)

---

### 3. Asset Detail Screen

```
┌─────────────────────────────────────────────────────────┐
│ ← Back    Bitcoin (BTC)                                 │
├─────────────────────────────────────────────────────────┤
│ ┌──────────────────┐  ┌──────────────────────────────┐ │
│ │ Your Balance     │  │ Market Price                 │ │
│ │ 2.45 BTC         │  │ $36,420.50                   │ │
│ │ $89,234.50       │  │ +2.3% (24h)                  │ │
│ │                  │  │                              │ │
│ │ [Send] [Receive] │  │ MCap: $720B  Vol: $32B      │ │
│ └──────────────────┘  └──────────────────────────────┘ │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Price Chart                                        │  │
│ │ [1D] [1W] [1M] [3M] [1Y] [ALL]                    │  │
│ │                                                    │  │
│ │                      ╱╲                            │  │
│ │                   ╱─╯  ╲╮                         │  │
│ │             ╱─╮╱─╯      ╰╮                        │  │
│ │        ╱───╯  ╯           ╰─╮                     │  │
│ │   ╱───╯                     ╰─╮                   │  │
│ │                                                    │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Your Transactions                      [View All]  │  │
│ │ ┌────────────────────────────────────────────────┐ │  │
│ │ │ Sent to 0x7a8f...  -0.5 BTC  Jan 15  Confirmed│ │  │
│ │ │ Received from ...  +1.2 BTC  Jan 12  Confirmed│ │  │
│ │ └────────────────────────────────────────────────┘ │  │
│ └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

### 4. Send Transaction Screen

```
┌─────────────────────────────────────────────────────────┐
│ ← Back    Send Bitcoin                                  │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ From                                               │  │
│ │ ┌────────────────────────────────────────────────┐ │  │
│ │ │ [v] Wallet 1 (2.45 BTC available)              │ │  │
│ │ └────────────────────────────────────────────────┘ │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ To                                                 │  │
│ │ ┌────────────────────────────────────────────────┐ │  │
│ │ │ 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb   📋 │ │  │
│ │ └────────────────────────────────────────────────┘ │  │
│ │ [Address Book] [Scan QR]                          │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Amount                                             │  │
│ │ ┌──────────────────────────┐  ┌─────────────────┐ │  │
│ │ │ 0.5                      │  │ BTC        [v] │ │  │
│ │ └──────────────────────────┘  └─────────────────┘ │  │
│ │ ≈ $18,210.25                           [Max]      │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Network Fee                                        │  │
│ │ ○ Slow ($0.50)      ~30 mins                      │  │
│ │ ● Standard ($1.25)  ~10 mins  ← Selected          │  │
│ │ ○ Fast ($2.50)      ~2 mins                       │  │
│ │ ○ Custom                                          │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Summary                                            │  │
│ │ You will send:     0.5 BTC                        │  │
│ │ Network fee:       0.00003 BTC                    │  │
│ │ ─────────────────────────────────────────────────  │  │
│ │ Total:             0.50003 BTC ($18,211.34)       │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│            [Cancel]              [Review & Send]        │
└─────────────────────────────────────────────────────────┘
```

---

### 5. VC Projects Marketplace

```
┌─────────────────────────────────────────────────────────┐
│ VC Projects                    [Search] [Filter] [Sort] │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ Filters: [All] [DeFi] [NFT] [Infrastructure] [Gaming]   │
│                                                          │
│ ┌──────────────────┐  ┌──────────────────┐             │
│ │ 🌟 Featured      │  │                  │             │
│ │                  │  │  Project Logo    │             │
│ │  [Project Logo]  │  │                  │             │
│ │                  │  └──────────────────┘             │
│ │  ChainFlow DeFi  │  ChainVault Protocol             │
│ │                  │                                   │
│ │  Next-gen DEX    │  Secure multi-chain vault        │
│ │  with AI routing │  solution for institutions       │
│ │                  │                                   │
│ │  💰 $5M raising  │  💰 $3M raising                  │
│ │  👥 Seed Round   │  👥 Series A                     │
│ │  📊 Ethereum     │  📊 Multi-chain                  │
│ │                  │                                   │
│ │  [View Details]  │  [View Details]                  │
│ └──────────────────┘  └──────────────────┘             │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Recently Added                                     │  │
│ │                                                    │  │
│ │ [Logo] CryptoAI Labs                              │  │
│ │        AI-powered trading bot platform            │  │
│ │        $2M • Pre-seed • 🔥 Hot                    │  │
│ │                                    [View Details] │  │
│ ├────────────────────────────────────────────────────┤  │
│ │ [Logo] GameFi Arena                               │  │
│ │        Web3 gaming tournament platform            │  │
│ │        $10M • Series A • ⚡ Closing Soon          │  │
│ │                                    [View Details] │  │
│ └────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

### 6. Project Detail Page

```
┌─────────────────────────────────────────────────────────┐
│ ← Back to Projects                                      │
├─────────────────────────────────────────────────────────┤
│ ┌──────────────┐                                        │
│ │ Project Logo │  ChainFlow DeFi                        │
│ └──────────────┘  Next-generation DEX with AI routing  │
│                                                          │
│ [Overview] [Team] [Financials] [Due Diligence] [Docs]   │
├─────────────────────────────────────────────────────────┤
│                                                          │
│ ┌─────────────────────┐  ┌────────────────────────┐    │
│ │ Raising             │  │ Valuation              │    │
│ │ $5,000,000          │  │ $20M pre-money         │    │
│ │                     │  │                        │    │
│ │ Stage: Seed Round   │  │ Min. Investment: $25K  │    │
│ └─────────────────────┘  └────────────────────────┘    │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ About                                              │  │
│ │                                                    │  │
│ │ ChainFlow is building the next generation of      │  │
│ │ decentralized exchanges powered by AI routing...  │  │
│ │                                                    │  │
│ │ Key Highlights:                                    │  │
│ │ • $2M in TVL within first month                   │  │
│ │ • 10,000+ active users                            │  │
│ │ • Audited by CertiK and Quantstamp               │  │
│ │ • Backed by Sequoia, a16z                         │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Team                                               │  │
│ │ ┌──────────────┬──────────────┬──────────────┐    │  │
│ │ │ [Photo]      │ [Photo]      │ [Photo]      │    │  │
│ │ │ John Doe     │ Jane Smith   │ Bob Johnson  │    │  │
│ │ │ CEO          │ CTO          │ CFO          │    │  │
│ │ │ Ex-Google    │ Ex-Coinbase  │ Ex-Goldman   │    │  │
│ │ └──────────────┴──────────────┴──────────────┘    │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Documents                                          │  │
│ │ 📄 Pitch Deck (PDF)                          [↓]  │  │
│ │ 📄 Financial Projections (XLSX)              [↓]  │  │
│ │ 📄 Smart Contract Audit - CertiK (PDF)       [↓]  │  │
│ │ 📄 Whitepaper (PDF)                          [↓]  │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│ ┌────────────────────────────────────────────────────┐  │
│ │ Risk Assessment                         Score: 7.5 │  │
│ │ ▓▓▓▓▓▓▓▓░░ High Quality                           │  │
│ │                                                    │  │
│ │ ✓ Smart contract audited                          │  │
│ │ ✓ Experienced team                                │  │
│ │ ✓ Product launched                                │  │
│ │ ⚠ Early stage, high risk                          │  │
│ └────────────────────────────────────────────────────┘  │
│                                                          │
│                          [Express Interest] [Contact]   │
└─────────────────────────────────────────────────────────┘
```

---

## Component Library

### Core Components

#### 1. Button Component

```typescript
// Button.tsx
interface ButtonProps {
  variant: 'primary' | 'secondary' | 'danger' | 'ghost';
  size: 'sm' | 'md' | 'lg';
  disabled?: boolean;
  loading?: boolean;
  icon?: React.ReactNode;
  children: React.ReactNode;
  onClick?: () => void;
}

// Visual variants:
// Primary:   Blue background, white text
// Secondary: Gray background, white text
// Danger:    Red background, white text
// Ghost:     Transparent, colored text with border
```

#### 2. Card Component

```typescript
// Card.tsx
interface CardProps {
  title?: string;
  subtitle?: string;
  headerAction?: React.ReactNode;
  children: React.ReactNode;
  padding?: 'none' | 'sm' | 'md' | 'lg';
  hover?: boolean; // Enable hover effect
}

// Visual:
// - Background: Level 1 (#1E293B)
// - Border: 1px solid rgba(255,255,255,0.1)
// - Border radius: --radius-lg
// - Optional hover lift effect
```

#### 3. Input Components

```typescript
// TextInput.tsx
interface TextInputProps {
  label?: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
  error?: string;
  helpText?: string;
  prefix?: React.ReactNode; // e.g., $ or crypto icon
  suffix?: React.ReactNode; // e.g., [Max] button
  disabled?: boolean;
}

// Select.tsx
interface SelectProps {
  label?: string;
  options: Array<{value: string, label: string, icon?: ReactNode}>;
  value: string;
  onChange: (value: string) => void;
}
```

#### 4. Data Display Components

```typescript
// StatCard.tsx
interface StatCardProps {
  label: string;
  value: string | number;
  subtitle?: string;
  change?: {
    value: number;
    percentage: number;
    timeframe: string;
  };
  icon?: React.ReactNode;
}

// AssetIcon.tsx
interface AssetIconProps {
  symbol: string; // BTC, ETH, etc.
  size: 'sm' | 'md' | 'lg';
  showBadge?: boolean; // Chain badge
}

// PriceChange.tsx
interface PriceChangeProps {
  value: number;
  percentage: number;
  timeframe?: string;
  showArrow?: boolean;
}
```

#### 5. Navigation Components

```typescript
// Sidebar.tsx
interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
  items: SidebarItem[];
}

interface SidebarItem {
  id: string;
  label: string;
  icon: React.ReactNode;
  path: string;
  badge?: string | number;
  children?: SidebarItem[];
}

// Tabs.tsx
interface TabsProps {
  items: Array<{id: string, label: string, count?: number}>;
  activeTab: string;
  onChange: (id: string) => void;
}
```

#### 6. Modal/Dialog Components

```typescript
// Modal.tsx
interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  size: 'sm' | 'md' | 'lg' | 'xl';
  children: React.ReactNode;
  footer?: React.ReactNode;
}

// ConfirmDialog.tsx
interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'default' | 'danger';
  onConfirm: () => void;
  onCancel: () => void;
}
```

#### 7. Chart Components

```typescript
// LineChart.tsx (using recharts or similar)
interface LineChartProps {
  data: Array<{timestamp: number, value: number}>;
  height?: number;
  showGrid?: boolean;
  color?: string;
  gradient?: boolean;
}

// PieChart.tsx
interface PieChartProps {
  data: Array<{name: string, value: number, color: string}>;
  size?: number;
  showLabels?: boolean;
}

// Sparkline.tsx (mini chart)
interface SparklineProps {
  data: number[];
  width: number;
  height: number;
  color: string;
}
```

---

## Frontend Architecture

### Project Structure

```
src/
├── components/
│   ├── ui/                      # Base UI components
│   │   ├── Button/
│   │   │   ├── Button.tsx
│   │   │   ├── Button.test.tsx
│   │   │   └── Button.module.css
│   │   ├── Card/
│   │   ├── Input/
│   │   ├── Modal/
│   │   └── index.ts
│   │
│   ├── features/                # Feature-specific components
│   │   ├── wallet/
│   │   │   ├── WalletCard.tsx
│   │   │   ├── TransactionList.tsx
│   │   │   └── SendForm.tsx
│   │   ├── portfolio/
│   │   │   ├── AssetTable.tsx
│   │   │   ├── PortfolioChart.tsx
│   │   │   └── AllocationPie.tsx
│   │   └── vc/
│   │       ├── ProjectCard.tsx
│   │       ├── ProjectDetail.tsx
│   │       └── InvestmentForm.tsx
│   │
│   ├── layout/                  # Layout components
│   │   ├── AppLayout.tsx
│   │   ├── Sidebar.tsx
│   │   ├── Header.tsx
│   │   └── PageHeader.tsx
│   │
│   └── common/                  # Shared components
│       ├── LoadingSpinner.tsx
│       ├── ErrorBoundary.tsx
│       └── EmptyState.tsx
│
├── pages/                       # Page components (routes)
│   ├── Dashboard.tsx
│   ├── Portfolio.tsx
│   ├── AssetDetail.tsx
│   ├── Transactions.tsx
│   ├── Markets.tsx
│   ├── Swap.tsx
│   ├── vc/
│   │   ├── Projects.tsx
│   │   ├── ProjectDetail.tsx
│   │   └── Investments.tsx
│   └── Settings.tsx
│
├── hooks/                       # Custom React hooks
│   ├── useWallet.ts
│   ├── usePortfolio.ts
│   ├── usePrices.ts
│   ├── useTransactions.ts
│   └── useProjects.ts
│
├── services/                    # API and external services
│   ├── api/
│   │   ├── wallet.service.ts
│   │   ├── price.service.ts
│   │   └── vc.service.ts
│   ├── blockchain/
│   │   ├── ethereum.service.ts
│   │   ├── bitcoin.service.ts
│   │   └── web3.service.ts
│   └── storage/
│       ├── database.service.ts
│       └── encryption.service.ts
│
├── store/                       # State management
│   ├── slices/
│   │   ├── walletSlice.ts
│   │   ├── portfolioSlice.ts
│   │   ├── pricesSlice.ts
│   │   └── vcSlice.ts
│   ├── store.ts
│   └── hooks.ts
│
├── utils/                       # Utility functions
│   ├── formatters.ts           # Currency, number formatting
│   ├── validators.ts           # Address, input validation
│   ├── calculations.ts         # Portfolio calculations
│   └── constants.ts
│
├── types/                       # TypeScript types
│   ├── wallet.types.ts
│   ├── portfolio.types.ts
│   ├── transaction.types.ts
│   └── vc.types.ts
│
├── styles/                      # Global styles
│   ├── globals.css
│   ├── variables.css
│   └── themes.css
│
├── App.tsx
└── main.tsx
```

### State Management Strategy

**Use Redux Toolkit for:**
- Global wallet state
- Portfolio data
- Price updates
- User settings

**Use React Query for:**
- API data fetching
- Caching price data
- Background updates
- Optimistic updates

**Use Local State for:**
- Form inputs
- UI state (modals, dropdowns)
- Temporary data

```typescript
// Example: walletSlice.ts
interface WalletState {
  wallets: Wallet[];
  activeWallet: string | null;
  isLoading: boolean;
  error: string | null;
}

// Example: usePrices hook with React Query
const usePrices = (symbols: string[]) => {
  return useQuery({
    queryKey: ['prices', symbols],
    queryFn: () => fetchPrices(symbols),
    refetchInterval: 30000, // 30 seconds
  });
};
```

---

## Tauri Integration

### Rust Backend Commands

```rust
// src-tauri/src/commands/wallet.rs

#[tauri::command]
async fn create_wallet(password: String) -> Result<WalletInfo, String> {
    // Generate new wallet
    // Encrypt private key
    // Store in database
}

#[tauri::command]
async fn import_wallet(
    mnemonic: String,
    password: String
) -> Result<WalletInfo, String> {
    // Import from seed phrase
}

#[tauri::command]
async fn get_balance(
    wallet_address: String,
    chain: String
) -> Result<Balance, String> {
    // Query blockchain for balance
}

#[tauri::command]
async fn send_transaction(
    from: String,
    to: String,
    amount: String,
    password: String
) -> Result<TransactionHash, String> {
    // Sign and broadcast transaction
}

#[tauri::command]
async fn get_transaction_history(
    wallet_address: String,
    chain: String
) -> Result<Vec<Transaction>, String> {
    // Fetch transaction history
}

#[tauri::command]
async fn encrypt_data(data: String, password: String) -> Result<String, String> {
    // Encrypt sensitive data
}

#[tauri::command]
async fn decrypt_data(encrypted: String, password: String) -> Result<String, String> {
    // Decrypt sensitive data
}
```

### Frontend-Backend Communication

```typescript
// services/tauri.service.ts
import { invoke } from '@tauri-apps/api/tauri';

export const walletService = {
  createWallet: async (password: string) => {
    return await invoke<WalletInfo>('create_wallet', { password });
  },

  getBalance: async (address: string, chain: string) => {
    return await invoke<Balance>('get_balance', {
      walletAddress: address,
      chain
    });
  },

  sendTransaction: async (params: SendTransactionParams) => {
    return await invoke<string>('send_transaction', params);
  }
};

// Usage in React component
const { mutate: createWallet, isLoading } = useMutation({
  mutationFn: (password: string) => walletService.createWallet(password),
  onSuccess: (wallet) => {
    // Update state
  }
});
```

---

## Data Flow Example: Send Transaction

```
User Action → Component → Hook → Tauri Command → Rust Backend → Blockchain
    ↓           ↓          ↓          ↓              ↓              ↓
  Click      SendForm   useSend   send_transaction  ethers-rs    Ethereum
  [Send]       ↓          ↓          ↓              Sign TX      Network
               ↓          ↓          ↓              Encrypt       ↓
           Validation  Loading    Parse Params    Store TX    Broadcast
               ↓          ↓          ↓              ↓              ↓
           [Review]   Show TX    Call Rust      Return Hash   Confirmation
               ↓          ↓          ↓              ↓              ↓
          [Confirm]   Execute    Response       Success/Error   Receipt
               ↓          ↓          ↓              ↓              ↓
            Success   Update UI  Update Store   Log to DB     Update UI
```

---

## Development Workflow

### Phase 1: Setup & Foundation (Week 1-2)

1. **Design System Setup**
   ```bash
   # Install dependencies
   npm install tailwindcss
   npm install @radix-ui/react-* # for accessible primitives
   npm install lucide-react # for icons
   npm install recharts # for charts
   ```

2. **Create Base Components**
   - Button, Card, Input, Select
   - Modal, Toast, Dropdown
   - Layout components (Sidebar, Header)

3. **Setup Routing**
   ```bash
   npm install react-router-dom
   ```

4. **Configure State Management**
   ```bash
   npm install @reduxjs/toolkit react-redux
   npm install @tanstack/react-query
   ```

### Phase 2: Wallet Features (Week 3-6)

1. **Implement Wallet Creation**
   - Generate mnemonic (Rust backend)
   - Encrypt and store
   - Backup flow UI

2. **Build Dashboard**
   - Portfolio summary
   - Charts integration
   - Real-time price updates

3. **Transaction Management**
   - Send/Receive forms
   - Transaction history
   - Address book

### Phase 3: Advanced Features (Week 7-10)

1. **Exchange Integration**
   - API connections
   - Trading interface
   - Order management

2. **DeFi Features**
   - Swap interface
   - Staking UI
   - Protocol integrations

### Phase 4: VC Platform (Week 11-16)

1. **Project Marketplace**
   - Project listing UI
   - Search and filters
   - Project detail page

2. **Investment Flow**
   - KYC integration
   - Investment forms
   - Document management

---

## Design Tools & Resources

### Recommended Tools

1. **Design**: Figma for mockups and prototypes
2. **Icons**: Lucide React or Heroicons
3. **Animations**: Framer Motion
4. **Charts**: Recharts or TradingView Lightweight Charts
5. **Forms**: React Hook Form + Zod

### Color Testing

Create a theme switcher early to test:
- Dark mode (primary)
- Light mode (future)
- High contrast mode (accessibility)

### Responsive Breakpoints

```css
--breakpoint-sm: 640px   /* Mobile */
--breakpoint-md: 768px   /* Tablet */
--breakpoint-lg: 1024px  /* Desktop */
--breakpoint-xl: 1280px  /* Large desktop */
```

---

## Testing Strategy

### Component Testing
```typescript
// Example: Button.test.tsx
import { render, fireEvent } from '@testing-library/react';
import { Button } from './Button';

test('button click handler', () => {
  const handleClick = jest.fn();
  const { getByText } = render(
    <Button onClick={handleClick}>Click me</Button>
  );

  fireEvent.click(getByText('Click me'));
  expect(handleClick).toHaveBeenCalledTimes(1);
});
```

### Integration Testing
- Test complete user flows (create wallet, send transaction)
- Mock Tauri commands
- Test state management

### E2E Testing
```typescript
// Using Playwright or similar
test('complete send transaction flow', async () => {
  await page.goto('/portfolio');
  await page.click('[data-testid="send-button"]');
  await page.fill('[data-testid="address-input"]', 'VALID_ADDRESS');
  await page.fill('[data-testid="amount-input"]', '0.1');
  await page.click('[data-testid="review-button"]');
  await page.click('[data-testid="confirm-button"]');

  await expect(page.locator('[data-testid="success-message"]')).toBeVisible();
});
```

---

## Performance Optimization

### Key Strategies

1. **Code Splitting**
   ```typescript
   const VCProjects = lazy(() => import('./pages/vc/Projects'));
   ```

2. **Memoization**
   ```typescript
   const sortedAssets = useMemo(() => {
     return assets.sort((a, b) => b.value - a.value);
   }, [assets]);
   ```

3. **Virtual Lists** for long transaction lists
   ```bash
   npm install react-virtual
   ```

4. **Image Optimization** for project logos, team photos

5. **Database Indexing** in SQLite for fast queries

---

## Accessibility

### Requirements

- WCAG 2.1 AA compliance
- Keyboard navigation for all actions
- Screen reader support
- Focus indicators
- Color contrast ratios > 4.5:1
- Alt text for images
- ARIA labels for interactive elements

```typescript
// Example: Accessible button
<button
  aria-label="Send Bitcoin"
  aria-describedby="send-help-text"
  disabled={isDisabled}
>
  Send
</button>
```

---

## Next Steps

### Immediate Actions

1. **Create Figma mockups** for key screens (Dashboard, Portfolio, Send)
2. **Set up Tailwind** with custom color palette
3. **Build component storybook** for design system
4. **Implement base layout** (Sidebar, Header, routing)
5. **Create mock data** for development

### Week 1 Goals

- [ ] Complete design system foundation
- [ ] Build 10 core UI components
- [ ] Implement app layout and navigation
- [ ] Set up Redux store structure
- [ ] Create Tauri command stubs

---

This implementation guide provides the technical foundation for building Aiigo Desktop with a focus on UI/UX design and developer workflow. Start with the foundation, iterate quickly, and build features progressively.
