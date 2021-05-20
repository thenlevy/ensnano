# Changelog

<!-- next-header -->

## Unreleased
- Fix a bug that would make user believe that a cross-over between two 3' ends or two 5' ends was possible in 3D
view
- Make it possible to put rotation/translation origin of grids at any point on the grid's latice
- Fix translation of phantom helices
- Add new icons for rotation and translation action modes, when these action are performed in the
object's coordinates
- Display the values of DNA parameters in the Parameters Tab
- Add a "new design" button
- Open a dialog that ask if the user want to save their design before closing the app or opening
an other design
- Fix import of scadnano files containing insertions or deletions & replace those by single strands
- Fix a bug that would cause the fitting of the 2D view to be incorrect
- Fix a bug that would incorretly initiate the length of strands when creating nanotubes
- Fix position of text indicating negative indices on helices in 2d view
- Fit design on loading
- Don't switch to Nucleotide selection mode when pasting

## 0.2.0
- Introduce `ensnano_version` and function to update older ensnano files
- Update default DNA parameters.
- Make it possible to undo rigid grids and rigid helices simulations

