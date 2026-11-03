# 05: Supply independent starters and a versioned contract bundle

Status: done

Blocked by: 04

## What to build

An author starts either a data-only Game or a Rust/WebAssembly Game in a
directory outside the Engine checkout, follows the supplied instructions, and
produces validated installable content without internal Engine dependencies.

## Acceptance criteria

- [ ] A reusable data-only starter demonstrates declared ground and appearance,
      includes needed artwork, and can be validated and packaged without
      invoking Rust or a WebAssembly build.
- [ ] A reusable Rust starter builds a component against its supplied versioned
      WIT bundle without Engine crate dependencies or paths into the checkout.
- [ ] Both starters include clear build, validate, package and install
      instructions and identify their supported contract line.
- [ ] The WIT bundle matches the supported public contract; the starter does not
      obtain types or behaviour through an internal Engine shortcut.
- [ ] Compatibility documentation distinguishes compatible patches from breaking
      contract lines, explains retained older-component support, and leaves the
      final 1.0 contract freeze to step 27.
- [ ] Templates and copied contract resources have explicit reuse-compatible
      licensing consistent with the existing demo and public Mod boundary.
- [ ] Automated verification copies each starter outside the checkout, builds
      the Rust project from the supplied bundle, validates and packages both,
      and loads their installed output through the normal content path.

## Verification

Use external temporary project directories so accidental relative paths into the
Engine are exposed. Record commands, test results and any setup needed for the
Rust toolchain when completing this ticket.
