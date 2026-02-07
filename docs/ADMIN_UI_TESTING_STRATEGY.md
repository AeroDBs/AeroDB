# Phase 13: Admin Dashboard - Testing Strategy

## Test Pyramid

```
        E2E Tests (20%)
       /            \
      /  Integration  \
     /    Tests (30%)  \
    /                   \
   /_____________________\
      Unit Tests (50%)
```

---

## Unit Tests (Vitest)

### What to Test

- **React Components**: Rendering, props, user interactions
- **Hooks**: Custom hooks (API calls, state management)
- **Utilities**: Helper functions, formatters, validators

### Example: Component Test

```typescript
// features/database/components/TableBrowser.test.tsx
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { TableBrowser } from './TableBrowser';

describe('TableBrowser', () => {
  it('renders table data', async () => {
    const queryClient = new QueryClient();
    
    render(
      <QueryClientProvider client={queryClient}>
        <TableBrowser collection="users" />
      </QueryClientProvider>
    );
    
    await waitFor(() => {
      expect(screen.getByText('users')).toBeInTheDocument();
    });
  });
  
  it('handles empty state', () => {
    render(<TableBrowser collection="empty_table" />);
    expect(screen.getByText('No data')).toBeInTheDocument();
  });
});
```

### Example: Hook Test

```typescript
// features/database/hooks/useTableData.test.ts
import { renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useTableData } from './useTableData';

describe('useTableData', () => {
  it('fetches table data', async () => {
    const queryClient = new QueryClient();
    const wrapper = ({ children }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    
    const { result } = renderHook(() => useTableData('users', { limit: 10, offset: 0 }), { wrapper });
    
    await waitFor(() => {
      expect(result.current.data).toBeDefined();
    });
  });
});
```

---

## Integration Tests (React Testing Library)

### What to Test

- **Feature flows**: Multi-step user journeys
- **API integration**: Components + API client
- **State management**: React Query + Zustand integration

### Example: Login Flow

```typescript
// features/auth/LoginPage.test.tsx
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { setupServer } from 'msw/node';
import { rest } from 'msw';
import { LoginPage } from './LoginPage';

const server = setupServer(
  rest.post('/auth/login', (req, res, ctx) => {
    return res(ctx.json({ access_token: 'fake-token' }));
  })
);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

describe('LoginPage', () => {
  it('logs in successfully', async () => {
    render(<LoginPage />);
    
    fireEvent.change(screen.getByLabelText('Email'), {
      target: { value: 'test@example.com' },
    });
    fireEvent.change(screen.getByLabelText('Password'), {
      target: { value: 'password123' },
    });
    fireEvent.click(screen.getByText('Login'));
    
    await waitFor(() => {
      expect(localStorage.getItem('access_token')).toBe('fake-token');
    });
  });
});
```

---

## E2E Tests (Playwright)

### What to Test

- **Critical paths**: Login → browse data → logout
- **Cross-page flows**: Create user → assign role → verify permissions
- **Visual regression**: Screenshots of key pages

### Example: Table Browsing Flow

```typescript
// tests/e2e/table-browsing.spec.ts
import { test, expect } from '@playwright/test';

test('browse table data', async ({ page }) => {
  // Login
  await page.goto('http://localhost:5173/login');
  await page.fill('input[name="email"]', 'admin@example.com');
  await page.fill('input[name="password"]', 'admin123');
  await page.click('button[type="submit"]');
  
  // Navigate to database section
  await page.click('text=Database');
  await page.waitForURL('**/database/tables');
  
  // Select a table
  await page.click('text=users');
  
  // Verify table data loaded
  await expect(page.locator('table')).toBeVisible();
  await expect(page.locator('tbody tr')).toHaveCount(20); // First page
  
  // Test pagination
  await page.click('text=Next');
  await expect(page.locator('tbody tr')).toHaveCount(20); // Second page
  
  // Test filtering
  await page.fill('input[placeholder="Search"]', 'john');
  await page.click('text=Apply Filter');
  await expect(page.locator('tbody tr')).toHaveCount.toBeLessThan(20);
  
  // Screenshot for visual regression
  await page.screenshot({ path: 'screenshots/table-browser.png' });
});
```

### Example: Real-Time Subscription

```typescript
// tests/e2e/realtime.spec.ts
import { test, expect } from '@playwright/test';

test('subscribe to real-time events', async ({ page }) => {
  await page.goto('http://localhost:5173/realtime/events');
  
  // Enable live updates
  await page.click('text=Enable Live Updates');
  
  // Verify WebSocket connection
  await expect(page.locator('text=Connected')).toBeVisible();
  
  // In another tab, insert data (simulate)
  await page.evaluate(() => {
    fetch('http://localhost:54321/rest/v1/posts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Authorization: 'Bearer token' },
      body: JSON.stringify({ title: 'New Post' }),
    });
  });
  
  // Verify event received
  await expect(page.locator('text=INSERT: posts')).toBeVisible({ timeout: 5000 });
});
```

---

## Visual Regression Testing

Use Playwright's screenshot comparison:

```typescript
// tests/e2e/visual.spec.ts
import { test, expect } from '@playwright/test';

test('homepage visual regression', async ({ page }) => {
  await page.goto('http://localhost:5173');
  await expect(page).toHaveScreenshot('homepage.png', {
    maxDiffPixels: 100,
  });
});
```

---

## API Mocking (MSW)

Use Mock Service Worker for deterministic tests:

```typescript
// src/mocks/handlers.ts
import { rest } from 'msw';

export const handlers = [
  rest.get('/rest/v1/users', (req, res, ctx) => {
    return res(
      ctx.json({
        data: [
          { id: 1, name: 'Alice', email: 'alice@example.com' },
          { id: 2, name: 'Bob', email: 'bob@example.com' },
        ],
        count: 2,
        limit: 20,
        offset: 0,
      })
    );
  }),
  
  rest.post('/auth/login', (req, res, ctx) => {
    const { email, password } = req.body as any;
    if (email === 'admin@example.com' && password === 'admin123') {
      return res(ctx.json({ access_token: 'mock-token' }));
    }
    return res(ctx.status(401), ctx.json({ error: 'Invalid credentials' }));
  }),
];

// src/mocks/browser.ts
import { setupWorker } from 'msw';
import { handlers } from './handlers';

export const worker = setupWorker(...handlers);

// Start in dev mode
if (import.meta.env.DEV) {
  worker.start();
}
```

---

## Coverage Targets

- **Unit tests**: 80% line coverage
- **Integration tests**: 70% feature coverage
- **E2E tests**: 100% critical path coverage

Run coverage:
```bash
npm run test:coverage
npm run test:e2e:coverage
```

---

## CI/CD Pipeline

### GitHub Actions

```yaml
# .github/workflows/test.yml
name: Test
on: [push, pull_request]

jobs:
  unit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: npm run test:unit
      - run: npm run test:coverage
  
  e2e:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: npx playwright install
      - run: npm run test:e2e
      - uses: actions/upload-artifact@v3
        if: failure()
        with:
          name: screenshots
          path: tests/e2e/screenshots/
```

---

## Performance Testing

### Lighthouse CI

```yaml
# .github/workflows/lighthouse.yml
name: Lighthouse
on: [push]

jobs:
  lighthouse:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: treosh/lighthouse-ci-action@v9
        with:
          urls: |
            http://localhost:5173
            http://localhost:5173/database
          uploadArtifacts: true
          temporaryPublicStorage: true
```

Targets:
- Performance score: > 90
- Accessibility score: 100
- Best Practices score: > 95

---

## Test Data Management

### Fixtures

```typescript
// tests/fixtures/users.ts
export const mockUsers = [
  { id: '1', name: 'Alice', email: 'alice@example.com', role: 'admin' },
  { id: '2', name: 'Bob', email: 'bob@example.com', role: 'user' },
];

// tests/fixtures/tables.ts
export const mockTables = [
  { name: 'users', count: 100 },
  { name: 'posts', count: 500 },
];
```

---

## Testing Best Practices

1. **Arrange-Act-Assert**: Structure tests clearly
2. **Test behavior, not implementation**: Avoid testing internal state
3. **Use data-testid sparingly**: Prefer accessible queries (getByRole, getByLabelText)
4. **Avoid flaky tests**: Use `waitFor`, avoid hardcoded timeouts
5. **Keep tests fast**: Mock external dependencies
