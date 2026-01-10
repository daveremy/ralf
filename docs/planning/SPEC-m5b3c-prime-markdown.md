# SPEC-m5b3c': Markdown Foundation

## Promise

AI messages in the timeline render with proper markdown styling (bold headers, code blocks, formatted lists) instead of showing literal markdown syntax. A shared text rendering module provides consistent markdown rendering across all components.

## Background

M5-B.3c introduced a simple line-by-line markdown parser for SpecPreview. While functional, this approach has limitations:

1. **No inline formatting** - Can't handle `**bold**` or `*italic*`
2. **Limited nesting** - Simple regex-based parsing misses edge cases
3. **Duplicate implementations** - Timeline will need its own markdown handling
4. **Plain text AI messages** - Currently shows literal `**Interface**:` instead of **Interface**:

Codex uses `pulldown-cmark`, a battle-tested Rust markdown parser that handles streaming, edge cases, and proper CommonMark compliance. We should adopt the same approach.

## Deliverables

### 1. Add `pulldown-cmark` Dependency
- Add `pulldown-cmark` to `ralf-tui/Cargo.toml`
- Version should match what Codex uses for compatibility reference

### 2. Create Shared `text/` Module
New module structure:
```
crates/ralf-tui/src/text/
├── mod.rs           # Public exports
├── markdown.rs      # pulldown-cmark based renderer
└── styles.rs        # MarkdownStyles configuration
```

**`markdown.rs`** provides:
```rust
/// Render markdown text to styled ratatui Lines.
pub fn render_markdown(input: &str, width: usize, theme: &Theme) -> Vec<Line<'static>>;

/// Append markdown to existing lines (for streaming).
pub fn append_markdown(input: &str, width: Option<usize>, lines: &mut Vec<Line<'static>>);
```

**`styles.rs`** provides:
```rust
pub struct MarkdownStyles {
    pub h1: Style,
    pub h2: Style,
    pub h3: Style,
    pub code: Style,
    pub code_block: Style,
    pub emphasis: Style,      // italic
    pub strong: Style,        // bold
    pub list_marker: Style,
    pub link: Style,
    pub blockquote: Style,
}

impl MarkdownStyles {
    pub fn from_theme(theme: &Theme) -> Self;
}
```

### 3. Update SpecPreview to Use Shared Renderer
- Remove `context/markdown.rs` (the simple parser)
- Update `SpecPreview` to use `text::render_markdown()`
- Maintain all existing functionality (phase badge, scrolling, etc.)

### 4. Add Markdown to ConversationPane
- AI/Assistant messages render with markdown styling
- User messages remain plain styled text (like Codex)
- System messages remain plain styled text

Message type detection:
```rust
match event {
    EventKind::Spec(SpecEvent { role: Role::Assistant, .. }) => {
        // Render with markdown
        render_markdown(&content, width, theme)
    }
    EventKind::Spec(SpecEvent { role: Role::User, .. }) => {
        // Plain styled text with user prefix
        render_user_message(&content, width, theme)
    }
    // ... other event types stay plain
}
```

## Non-Goals

- Syntax highlighting for code blocks (plain code styling is fine)
- Image rendering in markdown
- Link clicking/navigation
- Table rendering (not commonly used in AI responses)
- Custom markdown extensions
- Streaming markdown parsing (defer to future if needed)

## Acceptance Criteria

- [ ] `pulldown-cmark` added as dependency
- [ ] `text/` module created with `render_markdown()` function
- [ ] SpecPreview uses shared renderer (no regression in functionality)
- [ ] AI messages in timeline show styled headers (# → bold)
- [ ] AI messages show styled code blocks (``` → highlighted background)
- [ ] AI messages show styled inline code (`code` → highlighted)
- [ ] AI messages show styled bold/italic (**bold** → bold, *italic* → italic)
- [ ] AI messages show styled lists (- item → bullet point)
- [ ] User messages remain plain styled text (no markdown parsing)
- [ ] Old `context/markdown.rs` removed
- [ ] All existing tests pass
- [ ] New unit tests for markdown renderer

## Technical Approach

### pulldown-cmark Usage Pattern

Based on Codex's implementation:

```rust
use pulldown_cmark::{Parser, Event, Tag, Options};

pub fn render_markdown(input: &str, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(input, options);
    let styles = MarkdownStyles::from_theme(theme);

    let mut renderer = MarkdownRenderer::new(parser, width, styles);
    renderer.run();
    renderer.lines
}

struct MarkdownRenderer<I> {
    iter: I,
    lines: Vec<Line<'static>>,
    styles: MarkdownStyles,
    // Track current style stack for nested formatting
    style_stack: Vec<Style>,
    // Current line being built
    current_spans: Vec<Span<'static>>,
    // Indentation for lists
    indent_level: usize,
}
```

### Event Handling

Key events to handle:
- `Event::Start(Tag::Heading(level))` → Push heading style
- `Event::End(Tag::Heading(_))` → Flush line, pop style
- `Event::Start(Tag::CodeBlock(_))` → Enter code block mode
- `Event::Start(Tag::Emphasis)` → Push italic style
- `Event::Start(Tag::Strong)` → Push bold style
- `Event::Start(Tag::List(_))` → Increase indent
- `Event::Start(Tag::Item)` → Add bullet/number prefix
- `Event::Text(text)` → Add styled span
- `Event::Code(code)` → Add inline code span
- `Event::SoftBreak` → Space or newline depending on context
- `Event::HardBreak` → Flush current line

### Integration with ConversationPane

Update `conversation/widget.rs` to detect message type and render accordingly:

```rust
fn render_event(&self, event: &TimelineEvent, width: u16) -> Vec<Line<'static>> {
    match &event.kind {
        EventKind::Spec(spec) => match spec.role {
            Role::Assistant => text::render_markdown(&spec.content, width as usize, self.theme),
            Role::User => self.render_user_message(&spec.content, width),
        },
        EventKind::System(sys) => self.render_system_message(&sys.message, width),
        // ... other event types
    }
}
```

## Dependencies

- M5-B.3c (Spec Artifact View) - provides current markdown.rs to replace
- `pulldown-cmark` crate

## Testing

- Unit tests for `text::render_markdown()` covering all markdown elements
- Snapshot tests for rendered markdown output
- Integration test: send AI message with markdown → verify styled rendering
- Regression test: SpecPreview still works correctly

## Reference

- Codex `tui/src/markdown_render.rs` - their pulldown-cmark implementation
- Codex `tui/src/history_cell.rs` - how they render different message types
