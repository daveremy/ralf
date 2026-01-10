# SPEC-m5b3c: Spec Artifact View

## Promise

When a user is chatting with the AI to develop a spec, the right pane shows the extracted spec content with proper markdown rendering, updating live as the conversation progresses.

## Background

M5-B.3b implemented chat integration - users can now type messages and receive AI responses in the timeline. The AI extracts spec content from conversations using `extract_spec_from_response()`. Currently, the right pane shows a placeholder "(Implementation in M5-B.3)" instead of the actual spec.

This milestone renders the extracted spec in the context pane, providing visual feedback as the spec evolves through conversation.

## Deliverables

### 1. SpecPreview Widget
- New widget that renders spec content in the context pane
- Displays the current `Thread.draft` content
- Shows phase indicator (Drafting / Assessing / Finalized)
- Scrollable when content exceeds pane height

### 2. Markdown Rendering
- Headers (# ## ###) with visual hierarchy
- Code blocks (``` ```) with distinct background
- Inline code (`code`) styling
- Bullet lists (- *)
- Numbered lists (1. 2. 3.)
- Checkboxes (- [ ] and - [x])

### 3. Live Updates
- Spec updates as AI responses arrive
- Visual indication when spec is being updated (during chat loading)
- Smooth transition when new content arrives

### 4. Phase Display
- Clear indicator of current phase in the pane as compact badge
- `[Drafting]` - spec is being developed
- `[Assessing]` - AI is evaluating completeness (transient)
- `[Ready]` - spec is finalized, ready to run (distinct styling)

### 5. Artifact Actions (when context pane focused)
- `y` - Copy spec content to clipboard
- `e` - Edit spec (reverts to Drafting phase) [stub for now]

## Non-Goals

- Full markdown editor (this is view-only)
- Syntax highlighting for code blocks (plain styling is fine)
- Phase transitions triggered from this view (handled by chat/commands)
- Inline editing of spec content
- Images or links in markdown
- Inline formatting (bold, italic) - deferred to future enhancement

## Acceptance Criteria

- [ ] When thread has draft content, right pane shows rendered spec (not placeholder)
- [ ] Markdown headers render with visual hierarchy (larger/bold for h1, etc.)
- [ ] Code blocks render with distinct background color
- [ ] Lists render with proper indentation and bullets/numbers
- [ ] Checkboxes render as [ ] or [x] visual indicators
- [ ] Spec content is scrollable when it exceeds pane height
- [ ] Phase indicator shows current phase (Drafting/Assessing/Finalized)
- [ ] `y` key copies spec to clipboard when context pane is focused
- [ ] Spec updates live as AI responses arrive (no manual refresh needed)
- [ ] Empty state shows helpful message when no spec content yet

## Technical Approach

### Widget Structure
```
┌─ Spec ──────────────────────────────────┐
│ [Drafting]                              │
│                                         │
│ # Feature Name                          │
│                                         │
│ ## Overview                             │
│ Description of the feature...           │
│                                         │
│ ## Requirements                         │
│ - First requirement                     │
│ - Second requirement                    │
│   - [ ] Sub-item unchecked              │
│   - [x] Sub-item checked                │
│                                         │
│ ## Implementation                       │
│ ```rust                                 │
│ fn example() {                          │
│     // code here                        │
│ }                                       │
│ ```                                     │
└─────────────────────────────────────────┘
```

### Data Flow
```
Thread.draft (String)
  → parse_markdown()
  → Vec<MarkdownBlock>
  → SpecPreview widget
  → render to buffer
```

### Markdown Parser
Simple line-by-line parser that identifies:
- Header lines (starting with #)
- Code fence boundaries (```)
- List items (starting with - or *)
- Checkbox items (- [ ] or - [x])
- Regular paragraphs

No need for full AST - just enough to style appropriately.

## Dependencies

- M5-B.3b (Chat Integration) - provides Thread with draft content
- Existing theme colors for styling

## Testing

- Unit tests for markdown parser
- Snapshot tests for rendered output
- Integration test: send message → verify spec appears in pane
