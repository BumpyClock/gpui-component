## Plan
Replace WindowShell split placeholder behavior with a controlled resizing API and working drag interaction.

## Acceptance
- WindowShell exposes explicit left/right split width props.
- Split resize callback reports side plus width.
- Dragging splitters updates controlled widths through callback.
- No dead/placeholder split resize API remains.