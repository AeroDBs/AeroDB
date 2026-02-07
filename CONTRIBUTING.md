# Contributing to AeroDB

Thank you for your interest in contributing to AeroDB! We're building production-grade database infrastructure, and we value contributions that align with our core principles of correctness, predictability, and reliability.

## Code of Conduct

By participating in this project, you agree to maintain a professional, respectful environment focused on technical excellence.

## How to Contribute

### Reporting Bugs

**Before submitting a bug report:**
- Check the [existing issues](https://github.com/eshanized/AeroDB/issues) to avoid duplicates
- Verify the bug exists in the latest version
- Gather minimal reproduction steps

**When creating a bug report, include:**
- **Summary**: Clear one-line description
- **Environment**: OS, Rust version, AeroDB version
- **Reproduction Steps**: Minimal, complete code to reproduce
- **Expected vs Actual Behavior**: What should happen vs what does happen
- **Logs/Errors**: Relevant error messages or stack traces

### Suggesting Features

AeroDB follows strict design principles. Before suggesting features:

1. **Read the Vision**: Review [docs/CORE_VISION.md](docs/CORE_VISION.md)
2. **Check Alignment**: Ensure the feature aligns with our principles (determinism, schema-first, fail-fast)
3. **Search Existing Discussions**: Check if it's already been proposed

**Feature proposals should include:**
- **Problem Statement**: What problem does this solve?
- **Proposed Solution**: How should it work?
- **Alternatives Considered**: What other approaches did you consider?
- **Impact on Principles**: How does this affect determinism, correctness, etc?

## Development Workflow

### 1. Fork and Clone

```bash
git clone https://github.com/YOUR_USERNAME/AeroDB.git
cd AeroDB
git remote add upstream https://github.com/eshanized/AeroDB.git
```

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

**Branch naming conventions:**
- `feature/` - New features
- `fix/` - Bug fixes
- `docs/` - Documentation updates
- `refactor/` - Code refactoring
- `test/` - Test additions/improvements

### 3. Make Your Changes

#### Backend (Rust)

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy -- -D warnings

# Run tests
cargo test

# Build
cargo build --release
```

**Code Standards:**
- Follow Rust naming conventions (snake_case for functions/variables, PascalCase for types)
- Add documentation comments (`///`) for public APIs
- Write comprehensive error messages
- Include unit tests for new functionality

#### Frontend (Vue.js/TypeScript)

```bash
cd dashboard

# Install dependencies
npm install

# Development server
npm run dev

# Type checking
npm run build

# Run tests
npm test

# E2E tests
npm run test:e2e
```

**Code Standards:**
- Use TypeScript strict mode
- Follow Vue 3 Composition API patterns
- Use Pinia for state management
- Write component tests for new UI features

### 4. Write Tests

**All contributions must include tests:**

- **Backend**: Unit tests in `tests/` or inline tests
- **Frontend**: Component tests using Vitest and Vue Test Utils
- **Integration**: End-to-end tests if changing user workflows

**Test Naming:**
```rust
#[test]
fn test_feature_name_specific_behavior() {
    // Arrange
    // Act
    // Assert
}
```

### 5. Update Documentation

**If your changes affect:**
- **Public APIs**: Update `docs/CORE_API_SPEC.md`
- **User Behavior**: Update README.md
- **Architecture**: Update relevant `docs/PHASE*` files
- **Configuration**: Update setup wizard docs

### 6. Commit Your Changes

**Commit message format:**
```
type(scope): brief description

Longer explanation if needed

Closes #issue-number
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting changes
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

**Example:**
```
feat(auth): add password reset email workflow

Implements email-based password reset with secure token generation
and expiration. Tokens are single-use and expire after 1 hour.

Closes #123
```

### 7. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub.

## Pull Request Guidelines

### PR Title Format

Same as commit messages: `type(scope): description`

### PR Description Template

```markdown
## Summary
Brief overview of changes

## Motivation
Why is this change needed?

## Changes Made
- Change 1
- Change 2

## Testing
How was this tested?

## Checklist
- [ ] Tests pass (`cargo test` and `npm test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] Changelog entry added (if applicable)

## Related Issues
Closes #issue-number
```

### Review Process

1. **Automated Checks**: CI must pass (tests, linting, build)
2. **Code Review**: At least one maintainer approval required
3. **Testing**: Verify changes work as described
4. **Documentation**: Ensure docs are updated
5. **Merge**: Squash and merge when approved

## Code Review Standards

### What We Look For

✅ **Correctness**: Does it work as intended?
✅ **Testing**: Are edge cases covered?
✅ **Performance**: Any unnecessary performance degradation?
✅ **Safety**: Memory safety, error handling
✅ **Clarity**: Is the code readable and well-documented?
✅ **Consistency**: Matches existing code style

### What We Reject

❌ **Breaking Changes**: Without RFC and justification
❌ **Unsafe Code**: Unless absolutely necessary with clear safety comments
❌ **Magic Behavior**: Implicit assumptions, hidden side effects
❌ **Untested Code**: Missing tests for new functionality
❌ **Poor Error Handling**: `.unwrap()` abuse, unclear error messages

## Development Environment Setup

### Recommended Tools

**Rust:**
- [rust-analyzer](https://rust-analyzer.github.io/) - IDE support
- [cargo-watch](https://github.com/watchexec/cargo-watch) - Auto-rebuild on changes

**Frontend:**
- [Vue DevTools](https://devtools.vuejs.org/) - Browser extension
- [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) - VS Code extension

### Running Locally

```bash
# Terminal 1: Backend
cargo run -- serve

# Terminal 2: Frontend
cd dashboard && npm run dev

# Terminal 3: Tests (watch mode)
cargo watch -x test
```

## Architecture Decision Records (ADRs)

For significant changes, create an ADR:

```markdown
# ADR-XXX: Title

## Status
Proposed | Accepted | Rejected

## Context
What problem are we solving?

## Decision
What approach did we choose?

## Consequences
What are the trade-offs?
```

Save in `docs/adr-XXX-title.md`

## Community

- **Discussions**: [GitHub Discussions](https://github.com/eshanized/AeroDB/discussions)
- **Issues**: [GitHub Issues](https://github.com/eshanized/AeroDB/issues)

## Questions?

If you're unsure about anything, feel free to:
1. Open a discussion thread
2. Comment on a relevant issue
3. Ask in your PR description

We value thoughtful questions and constructive discussion.

---

**Thank you for contributing to AeroDB!** Together, we're building infrastructure that engineers can trust.
