# Contributing to Thaleia

Thank you for your interest in contributing to Thaleia! This document outlines the contribution process and licensing terms.

**New to Thaleia?** Start with:
- [README.md](./README.md) - What Thaleia is
- [PLAN.md](./PLAN.md) - Current progress
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Target architecture

## License Terms

By contributing to Thaleia, you agree that your contributions will be licensed under:

1. **GNU Affero General Public License v3 (AGPLv3)** - See [LICENSE](./LICENSE)
2. **Commercial License** - See [LICENSE-COMMERCIAL](./LICENSE-COMMERCIAL)

This means your contributions can be used:
- Freely under AGPLv3 for open source projects
- Commercially with a purchased commercial license

## Developer Certificate of Origin (DCO)

To ensure clear copyright ownership, we require a Developer Certificate of Origin (DCO) for all contributions.

### What is a DCO?

The DCO is a lightweight way to certify that you have the right to contribute the code. It's similar to the DCO used by the Linux Kernel and many other major open source projects.

### How to Sign Your Commits

Add a Signed-off-by line to every commit message:

```
Signed-off-by: Your Name <your@email.com>
```

This can be done automatically by using the `-s` or `--signoff` flag with git:

```bash
git commit -s -m "Your commit message"
```

### What the DCO Certifies

By adding a Signed-off-by line, you certify that:

1. You have the right to submit the contribution on behalf of the copyright owner
2. The contribution is your original work, OR you have the right to contribute it
3. You are submitting the contribution under the same license as the project (AGPLv3 + Commercial)

## Pull Request Process

1. **Fork the repository** and create a feature branch
2. **Make your changes** with proper commit messages
3. **Sign your commits** using `git commit -s`
4. **Test your changes** - ensure all tests pass
5. **Update documentation** if needed
6. **Submit a Pull Request** with a clear description

## Code Style

- Follow the existing code style in the project
- Run `cargo fmt` before committing
- Run `cargo clippy` and resolve any warnings
- Add tests for new functionality

## Community Contributions

Interested in contributing a new feature? Great! Please:

1. **Open discussion first**: Create an issue before starting work
2. **Coordinate**: Ensure no duplicate effort with other contributors
3. **Follow our workflow**: Use our templates and processes
4. **Be collaborative**: Be polite and welcoming to all contributors

### First Opportunity: Polish Voice Pack

We're seeking community help to create a **Polish voice pack** for Kokoro TTS. See [PLAN.md](./PLAN.md#community-contribution-polish-voice-pack) for details.

Requirements:
- 20-50 hours of permissively-licensed Polish audio
- GPU access for training (~$200-400)
- Experience with ML training

**Process**: Open issue → Wait for acknowledgment → Follow workflow → Submit PR

## Questions?

For questions about contributing or licensing:
- Open source issues: https://codeberg.org/ConYel/thaleia/issues
- Commercial licensing: ask maintainer

---

**Thank you for contributing to Thaleia!**
