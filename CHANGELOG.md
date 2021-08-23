# Changelog

<!-- next-header -->
## 0.3.0
- It is now possible to translate several helices at once.
- It is now possible to change the color of several strands at once. Moreover, the red hilighting
of the select strands disappears when changing color.
- Several actions that were not undoable before can now be undone:
 * selections
 * changing color of strands
 * moving helices in the 2d view
- A "Save As" button has been added with the behavior of the previous "Save" button. The "Save" button now
save the design with the current file name instead of opening a file picking dialog.

## 0.2.1
- Take loops into account when importing cadnano file
- Draw a cone pointing in the 3' direction at the 3' end of strands in 3D view.
- Improve automatic positioning of camera
- Fix a bug in oxdna export
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

