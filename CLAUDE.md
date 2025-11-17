# CLAUDE.md - AI Assistant Guide for Aiigo Desktop

## Project Overview

**Aiigo Desktop** is a cross-platform cryptocurrency wallet and venture capital platform built with Tauri, React, and TypeScript. It provides users with secure wallet management for Bitcoin and EVM-compatible chains, portfolio tracking, and access to a VC marketplace for blockchain projects.

### Project Identity
- **Name**: aiigo-desktop
- **Version**: 0.1.0
- **Identifier**: com.trumpcyka.aiigo-desktop
- **License**: Private

### Core Features
1. Multi-chain wallet management (Bitcoin, EVM chains)
2. Portfolio tracking and analytics
3. Transaction management
4. VC project marketplace (planned)
5. Secure local storage with SQLite

---

## Technology Stack

### Frontend
- **Framework**: React 19.1.0 with TypeScript
- **Build Tool**: Vite 7.0.4
- **Routing**: React Router DOM 7.9.4
- **Styling**: TailwindCSS 4.1.15
- **UI Components**: Radix UI primitives + custom components
- **Forms**: React Hook Form 7.65.0 + Zod 4.1.12
- **Charts**: Recharts 2.15.4
- **State Management**: Zustand 5.0.8
- **Icons**: Lucide React 0.546.0
- **Notifications**: Sonner 2.0.7

### Backend (Rust)
- **Framework**: Tauri v2
- **Database**: SQLite (via rusqlite)
- **Wallet Support**:
  - Bitcoin wallet functionality
  - EVM wallet functionality
- **Plugins**:
  - tauri-plugin-opener
  - tauri-plugin-window-state

### Development Tools
- **TypeScript**: ~5.8.3
- **Node**: v24.9.1+ (types)
- **Path Aliases**: `@/*` maps to `./src/*`

---

## Repository Structure

```
aiigo-desktop/
├── src/                          # Frontend React application
│   ├── components/
│   │   ├── common/              # Shared components (Sidebar, AppHeader)
│   │   ├── layout/              # Layout components (AppLayout)
│   │   └── ui/                  # Base UI components (shadcn/ui style)
│   ├── pages/                   # Route pages
│   │   ├── Dashboard/
│   │   ├── Portfolio/
│   │   └── NotFound.tsx
│   ├── hooks/                   # Custom React hooks
│   │   └── use-mobile.ts
│   ├── lib/                     # Utility functions
│   │   └── utils.ts            # cn(), shortAddress()
│   ├── App.tsx                  # Router configuration
│   ├── main.tsx                 # React entry point
│   └── vite-env.d.ts
│
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── wallet/
│   │   │   ├── bitcoin/        # Bitcoin wallet implementation
│   │   │   ├── evm/            # EVM wallet implementation
│   │   │   ├── types.rs        # Shared wallet types
│   │   │   └── mod.rs
│   │   ├── db.rs               # SQLite database layer
│   │   ├── lib.rs              # Tauri app initialization
│   │   └── main.rs             # Entry point
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── IMPLEMENTATION_GUIDE.md      # Detailed technical implementation guide
├── README.md
├── package.json
├── tsconfig.json
└── vite.config.ts
```

---

## Development Environment Setup

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js (v18+)
# Use nvm, fnm, or download from nodejs.org

# Install dependencies
npm install
```

### Running the Application

```bash
# Development mode (hot reload)
npm run dev

# Build for production
npm run build

# Preview production build
npm run preview

# Tauri commands
npm run tauri dev      # Run Tauri development
npm run tauri build    # Build Tauri application
```

### Development Server
- **Frontend**: http://localhost:1420
- **HMR**: Port 1421
- **Strict Port**: Yes (will fail if port unavailable)

---

## Architecture and Design Patterns

### Application Architecture

```
┌─────────────────────────────────────────┐
│         React Frontend (Vite)           │
│  ┌───────────────────────────────────┐  │
│  │  Pages (Routes)                   │  │
│  │  ├─ Dashboard                     │  │
│  │  ├─ Portfolio                     │  │
│  │  └─ NotFound                      │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  Components                       │  │
│  │  ├─ Layout (AppLayout)            │  │
│  │  ├─ Common (Sidebar, Header)      │  │
│  │  └─ UI (Radix primitives)         │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  State (Zustand)                  │  │
│  └───────────────────────────────────┘  │
└──────────────┬──────────────────────────┘
               │ IPC (invoke)
┌──────────────▼──────────────────────────┐
│         Rust Backend (Tauri)            │
│  ┌───────────────────────────────────┐  │
│  │  Tauri Commands                   │  │
│  │  ├─ Bitcoin Wallet Commands       │  │
│  │  ├─ EVM Wallet Commands           │  │
│  │  └─ Database Commands             │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  Business Logic                   │  │
│  │  ├─ Wallet Management             │  │
│  │  ├─ Mnemonic Generation           │  │
│  │  ├─ Private Key Management        │  │
│  │  └─ Balance Fetching              │  │
│  └───────────────────────────────────┘  │
│  ┌───────────────────────────────────┐  │
│  │  SQLite Database                  │  │
│  │  (Local encrypted storage)        │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

### Layout Structure

```
┌─────────────────────────────────────────┐
│  AppHeader (Fixed)                      │
├──────────┬──────────────────────────────┤
│          │                              │
│ Sidebar  │  Main Content (Outlet)       │
│ (Fixed)  │  ├─ Dashboard                │
│          │  ├─ Portfolio                │
│          │  └─ Other Pages              │
│          │                              │
└──────────┴──────────────────────────────┘
```

### Data Flow Pattern

```
User Action → Component → Tauri Command → Rust Handler → Database/Blockchain
    ↓            ↓            ↓                ↓              ↓
  Click       Handler      invoke()         Process         Query
    ↓            ↓            ↓                ↓              ↓
  Event      setState     Command          Business         Return
    ↓            ↓            ↓                ↓              ↓
  Render      Update        Result          Response        Update UI
```

---

## Key Conventions and Best Practices

### File Naming
- **React Components**: PascalCase (e.g., `AppLayout.tsx`, `Sidebar.tsx`)
- **Utilities**: camelCase (e.g., `utils.ts`, `use-mobile.ts`)
- **Rust Files**: snake_case (e.g., `db.rs`, `wallet.rs`)
- **Index Files**: `index.tsx` for page entry points

### Code Style

#### TypeScript/React
```typescript
// Use functional components with TypeScript
const ComponentName: React.FC<Props> = ({ prop1, prop2 }) => {
  // Component logic
  return <div>...</div>;
};

// Export named or default based on context
export default ComponentName;
export { ComponentName };
```

#### Import Organization
```typescript
// 1. External dependencies
import React from 'react';
import { useNavigate } from 'react-router-dom';

// 2. Internal components/utilities (use @ alias)
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

// 3. Local imports
import SubComponent from './SubComponent';
```

#### Styling
```typescript
// Use cn() utility for className merging
import { cn } from '@/lib/utils';

<div className={cn(
  "base-classes",
  conditionalClass && "conditional-classes",
  className
)} />
```

### Rust Conventions

```rust
// Module organization
pub mod submodule;
use crate::module::submodule;

// Tauri commands
#[tauri::command]
pub async fn command_name(param: Type) -> Result<ReturnType, String> {
    // Implementation
    Ok(result)
}

// Error handling
.map_err(|e| format!("Error description: {}", e))?
```

### Component Patterns

#### UI Components (shadcn/ui style)
Located in `src/components/ui/`, these are reusable primitives:
- Built on Radix UI
- Styled with TailwindCSS
- Use `cn()` for className composition
- Export as named exports

#### Layout Components
Located in `src/components/layout/`:
- Handle page structure
- Use `<Outlet />` for nested routes
- Manage global layout state

#### Common Components
Located in `src/components/common/`:
- Shared across features
- Not pure UI primitives
- Business logic aware

### State Management

```typescript
// Zustand store pattern
import { create } from 'zustand';

interface StoreState {
  value: string;
  setValue: (value: string) => void;
}

export const useStore = create<StoreState>((set) => ({
  value: '',
  setValue: (value) => set({ value }),
}));

// Usage in components
const { value, setValue } = useStore();
```

---

## Tauri Integration Patterns

### Invoking Rust Commands

```typescript
import { invoke } from '@tauri-apps/api/tauri';

// Basic invocation
const result = await invoke<ReturnType>('command_name', {
  paramName: value,
});

// With error handling
try {
  const wallets = await invoke<Wallet[]>('bitcoin_get_wallets');
  setWallets(wallets);
} catch (error) {
  console.error('Failed to fetch wallets:', error);
}
```

### Available Tauri Commands

#### Bitcoin Wallet Commands
- `bitcoin_create_mnemonic()` - Generate new mnemonic
- `bitcoin_import_mnemonic(mnemonic)` - Import from mnemonic
- `bitcoin_create_wallet_from_mnemonic(mnemonic, name)` - Create wallet
- `bitcoin_create_wallet_from_private_key(private_key, name)` - Import from private key
- `bitcoin_export_mnemonic(address)` - Export mnemonic
- `bitcoin_export_private_key(address)` - Export private key
- `bitcoin_get_wallets()` - Get all wallets
- `bitcoin_get_wallet(address)` - Get specific wallet
- `bitcoin_get_wallet_with_balance(address)` - Get wallet with balance
- `bitcoin_delete_wallet(address)` - Delete wallet

#### EVM Wallet Commands
- `evm_create_mnemonic()` - Generate new mnemonic
- `evm_import_mnemonic(mnemonic)` - Import from mnemonic
- `evm_create_wallet_from_mnemonic(mnemonic, name)` - Create wallet
- `evm_create_wallet_from_private_key(private_key, name)` - Import from private key
- `evm_export_mnemonic(address)` - Export mnemonic
- `evm_export_private_key(address)` - Export private key
- `evm_get_wallets()` - Get all wallets
- `evm_get_wallet(address)` - Get specific wallet
- `evm_get_wallet_with_balances(address)` - Get wallet with balances
- `evm_delete_wallet(address)` - Delete wallet

### Database Storage

The application uses SQLite for local storage:
- **Development**: `aiigo_debug.db` (root directory)
- **Production**:
  - macOS: `~/Library/Application Support/aiigo_desktop/wallets.db`
  - Windows: `%APPDATA%/aiigo_desktop/wallets.db`
  - Linux: `~/.local/share/aiigo_desktop/wallets.db`

---

## Common Tasks and Workflows

### Adding a New Page

1. Create page component in `src/pages/`:
```typescript
// src/pages/NewPage/index.tsx
import React from 'react';

const NewPage: React.FC = () => {
  return (
    <div className="p-6">
      <h1>New Page</h1>
    </div>
  );
};

export default NewPage;
```

2. Add route in `src/App.tsx`:
```typescript
import NewPage from './pages/NewPage';

const router = createBrowserRouter([
  {
    path: '/',
    element: <AppLayout />,
    children: [
      // ... existing routes
      { path: '/new-page', element: <NewPage /> },
    ],
  },
]);
```

3. Add navigation link in `src/components/common/Sidebar.tsx`

### Adding a New UI Component

1. Create component in `src/components/ui/`:
```typescript
// src/components/ui/new-component.tsx
import * as React from 'react';
import { cn } from '@/lib/utils';

interface NewComponentProps {
  className?: string;
  children?: React.ReactNode;
}

const NewComponent = React.forwardRef<HTMLDivElement, NewComponentProps>(
  ({ className, children, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("base-classes", className)}
        {...props}
      >
        {children}
      </div>
    );
  }
);
NewComponent.displayName = "NewComponent";

export { NewComponent };
```

2. Export from `src/components/ui/index.tsx`

### Adding a Tauri Command

1. Create command in Rust (e.g., `src-tauri/src/wallet/bitcoin/commands.rs`):
```rust
#[tauri::command]
pub async fn new_bitcoin_command(param: String) -> Result<String, String> {
    // Implementation
    Ok(result)
}
```

2. Register command in `src-tauri/src/lib.rs`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands
    bitcoin_commands::new_bitcoin_command,
])
```

3. Use in frontend:
```typescript
import { invoke } from '@tauri-apps/api/tauri';

const result = await invoke<string>('new_bitcoin_command', {
  param: 'value',
});
```

### Adding a Utility Function

Add to `src/lib/utils.ts`:
```typescript
export function newUtilityFunction(input: string): string {
  // Implementation
  return output;
}
```

---

## Testing Guidelines

### Component Testing (To Be Implemented)
```typescript
// Example pattern for future tests
import { render, screen } from '@testing-library/react';
import Component from './Component';

describe('Component', () => {
  it('renders correctly', () => {
    render(<Component />);
    expect(screen.getByText('Expected Text')).toBeInTheDocument();
  });
});
```

### Manual Testing Checklist
- [ ] Test on all target platforms (macOS, Windows, Linux)
- [ ] Verify wallet creation/import flows
- [ ] Test balance fetching
- [ ] Verify database persistence
- [ ] Check responsive design
- [ ] Test navigation flows

---

## Important Notes for AI Assistants

### When Working with This Codebase

1. **Always Use Path Aliases**: Use `@/` prefix for imports from `src/`
   ```typescript
   // Good
   import { Button } from '@/components/ui/button';

   // Avoid
   import { Button } from '../../components/ui/button';
   ```

2. **Follow Existing Patterns**: Match the style of existing code
   - UI components follow shadcn/ui patterns
   - Pages use similar structure to existing pages
   - Tauri commands follow established error handling

3. **Type Safety**: Always use TypeScript types
   ```typescript
   // Define types for Tauri command responses
   interface Wallet {
     address: string;
     name: string;
   }

   const wallets = await invoke<Wallet[]>('bitcoin_get_wallets');
   ```

4. **Styling Conventions**:
   - Use TailwindCSS utility classes
   - Use `cn()` for conditional classes
   - Follow mobile-first responsive design
   - Maintain dark theme compatibility

5. **Component Organization**:
   - UI primitives → `src/components/ui/`
   - Feature components → `src/components/[feature]/`
   - Shared components → `src/components/common/`
   - Layout components → `src/components/layout/`

6. **Rust Best Practices**:
   - All commands should return `Result<T, String>`
   - Use descriptive error messages
   - Follow module organization in `wallet/`
   - Update command registration in `lib.rs`

7. **State Management**:
   - Use Zustand for global state
   - Keep component-local state when appropriate
   - Consider using React Query for API data (if implemented)

8. **Security Considerations**:
   - Private keys must be encrypted
   - Never log sensitive data
   - Validate all user inputs
   - Use secure RNG for key generation

9. **Performance**:
   - Use React.memo for expensive components
   - Lazy load routes when appropriate
   - Optimize re-renders with proper dependencies
   - Consider virtualizing long lists

10. **Documentation**:
    - Add JSDoc comments for complex functions
    - Update this CLAUDE.md when adding major features
    - Keep IMPLEMENTATION_GUIDE.md in sync with architecture changes

### Common Pitfalls to Avoid

1. **Don't** import from relative paths when `@/` alias is available
2. **Don't** forget to register new Tauri commands in `lib.rs`
3. **Don't** use `any` type - always define proper types
4. **Don't** forget to handle errors in async operations
5. **Don't** mutate state directly - use proper state setters
6. **Don't** hardcode values - use constants or config
7. **Don't** forget mobile responsiveness
8. **Don't** skip error handling in Rust commands

### Quick Reference Commands

```bash
# Development
npm run dev                    # Start frontend dev server
npm run tauri dev             # Start Tauri development mode

# Building
npm run build                 # Build frontend
npm run tauri build          # Build Tauri app

# Code Quality
tsc                           # Type check TypeScript
npm run preview              # Preview production build

# Tauri Specific
cargo check --manifest-path src-tauri/Cargo.toml    # Check Rust code
cargo build --manifest-path src-tauri/Cargo.toml    # Build Rust
```

---

## Resources and References

### Documentation
- [Tauri v2 Docs](https://tauri.app/v2/)
- [React Router v7](https://reactrouter.com/)
- [TailwindCSS](https://tailwindcss.com/)
- [Radix UI](https://www.radix-ui.com/)
- [Zustand](https://github.com/pmndrs/zustand)
- [Recharts](https://recharts.org/)

### Internal Documentation
- `IMPLEMENTATION_GUIDE.md` - Detailed technical implementation guide
- `README.md` - Project overview and setup
- Component comments and JSDoc

### Project-Specific Patterns
- **Wallet Structure**: Study `src-tauri/src/wallet/` for wallet implementation patterns
- **Component Patterns**: Reference `src/components/ui/` for UI component structure
- **Layout Patterns**: See `src/components/layout/AppLayout.tsx` for layout approach
- **Utility Patterns**: Check `src/lib/utils.ts` for utility function patterns

---

## Version History

- **v0.1.0** (Current) - Initial development version
  - Basic wallet management (Bitcoin + EVM)
  - Portfolio page
  - Dashboard page
  - Core UI components
  - SQLite integration

---

## Contributing Guidelines

When making changes to this codebase:

1. **Maintain Consistency**: Follow existing patterns and conventions
2. **Type Everything**: Ensure all TypeScript is properly typed
3. **Test Locally**: Test on your platform before committing
4. **Update Documentation**: Update this file and IMPLEMENTATION_GUIDE.md as needed
5. **Error Handling**: Always handle errors gracefully
6. **Security First**: Be mindful of security implications, especially with wallet operations

---

**Last Updated**: 2025-11-17
**Maintainer**: Aiigo Desktop Team
