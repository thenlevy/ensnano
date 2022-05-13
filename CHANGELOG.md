# Changelog

## 0.5.0
- Introduce curved DNA strands in ENSnano
- Add a new DNA parmaters: inclination, and make it possible to change the DNA paramters

## 0.4.1
- Introduce the possibility to copy and paste whole grids with their content
- Make the strand color picker appear when the current selection contains a strand
(previously, it appeared when the current selection mode was strand)
- Treat unpaired nucleotide as seperate domains during staples export
- Show a question mark in place of letter for nucleotide with no basis in 2D view
- Use cursor icons to indicate that actions are being performed in the 2D/3D views
- Make it possible to tilt the 3D view with shift + RClick, and the 2D view with alt + Left/Right
- Make it possible to apply a symetry transformation to the 2d view by pressing ctrl/⌘ + Left/Up/Right/Down
- Make it possible to add single stranded insertions (a.k.a. loopouts) in strands
- Modify the behaviour of 3D camera rotations. RClick + drag has now the following effect depending of the
   active modifiers (in order of priority)
   * shift: tilt the camera
   * ctrl/⌘: Make a free rotation
   * alt: Make a rotation that preserver the current xz plane of the camera
   * nothing: Make a rotation that forces the xz plane of the camera to be the plane with equation y=0
- Improve the error message that appears when failing to open a design
- Make it possible to delete grids

## 0.4.0
- Remaps mouse buttons in the 2D view.
 * Left Clicking on a nucleotide selects it, clicking again selects the strands on which the nucleotide lies
 * Right Clicking on a nucleotide cuts the strands at that position, or merge it with the neighboring strand (this was the previous behavior of Left Clicking on a nucleotide)
 * Double Left clicking on a nucleotide center the nucleotide in the 3D view (previously this was done by double right clicking)
- Make it possible to move several domain ends at once.

## 0.3.2
- Use jemalloc alocator to prevent crash in macOS
- Add a contextual pannel to position objects in space
- Load immediatly scaffold sequence when loading a file
- Fix the filter for scaffold sequence files
- Always show the position of the hovered nucleotide in the 2D view

## 0.3.1
- Improve flexibility of the cross-over suggestions interface. The parameters are in the "Eddition" tab
- Update wgpu to 0.11
- Files that do not have an `.ens` extensions won't be overiden when saving a design. This fixes a problem that caused
ENSnano to overide cadnano files for example.
- Selected/candidate nucleotide are now highlighted in the 2D view.
- Fix a bug that would cause high CPU usage while ENSnano was in the background on MacOS

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
- Shows the length of the currently edited domain in the 2d view
- When editing a domain, its length (in bp and in nm) is now displayed in the 2d view
- The `wgpu` dependency is updated to `0.10.1`
- It is now possible to move helices in the 3d view by doing by grabing and draging the disc at the
intersection between the grid and the helix
- Strands can now be given a name for spreadsheet export
- Make it possible to set a pivot point for a group and for current selection
- Make it possible to create camera shortcuts 

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

