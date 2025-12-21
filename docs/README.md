# Hippo Documentation

This directory contains the source for Hippo's GitHub Pages documentation site.

## Local Development

To run the documentation site locally:

### Prerequisites

- Ruby 3.1 or higher
- Bundler

### Setup

```bash
# Install Ruby (if not already installed)
# macOS
brew install ruby

# Linux (Ubuntu/Debian)
sudo apt-get install ruby-full

# Install bundler
gem install bundler

# Install dependencies
cd docs
bundle install
```

### Running Locally

```bash
# Start Jekyll server
bundle exec jekyll serve

# Or with live reload
bundle exec jekyll serve --livereload

# Access at http://localhost:4000
```

### Building

```bash
# Build the site (output to _site/)
bundle exec jekyll build

# Build with production settings
JEKYLL_ENV=production bundle exec jekyll build
```

## Documentation Structure

```
docs/
├── _config.yml           # Jekyll configuration
├── Gemfile               # Ruby dependencies
├── index.md              # Home page
├── installation.md       # Installation guide
├── cli-guide.md          # CLI commands reference
├── desktop-app.md        # Desktop app guide
├── api.md                # API reference
├── architecture.md       # Architecture deep dive
└── contributing.md       # Contributing guide
```

## Theme

We use [Just the Docs](https://just-the-docs.github.io/just-the-docs/) theme with dark mode enabled.

## Deployment

Documentation is automatically deployed to GitHub Pages when changes are pushed to `main` branch via GitHub Actions.

Workflow: `.github/workflows/github-pages.yml`

## Writing Documentation

### Front Matter

Every page should include front matter:

```yaml
---
layout: default
title: Page Title
nav_order: 1
description: "Brief description"
---
```

### Navigation Order

Pages are ordered by `nav_order` in the front matter:

1. Home (index.md)
2. Installation
3. CLI Guide
4. Desktop App
5. API Reference
6. Architecture
7. Contributing

### Markdown Features

- Use `{: .no_toc }` to exclude headings from TOC
- Use `{: .fs-9 }` for larger text
- Use code blocks with language specifiers
- Use tables for structured data

### Code Examples

````markdown
```rust
// Rust code example
let hippo = Hippo::new().await?;
```

```bash
# Shell commands
hippo chomp ~/Documents
```

```javascript
// JavaScript examples
const result = await invoke('search', { query });
```
````

### Callouts

```markdown
{: .note }
Important information here

{: .warning }
Warning message here

{: .highlight }
Highlighted content
```

## Links

- [GitHub Pages Site](https://greplabs.github.io/hippo/)
- [Just the Docs Documentation](https://just-the-docs.github.io/just-the-docs/)
- [Jekyll Documentation](https://jekyllrb.com/docs/)

## Contributing to Docs

1. Fork the repository
2. Create a feature branch
3. Make your changes to docs/
4. Test locally with `bundle exec jekyll serve`
5. Submit a Pull Request

See [Contributing Guide](contributing.md) for more details.

---

Built with Jekyll and Just the Docs theme.
