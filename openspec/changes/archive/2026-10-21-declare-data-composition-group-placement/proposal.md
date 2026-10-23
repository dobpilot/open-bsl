# Why
Original BUH configuration compilation reaches DataCompositionGroupPlacement.BeginAndEnd in the AccountAnalysis report manager. The system enum is absent. The next replay reaches DataCompositionTextPlacementType.Wrap in the same report manager.

# What Changes
Declare the reached group-placement and text-placement enums, both language aliases, and measured type and value presentations using the existing system-enum tables.

# Impact
Runtime system enum and type tables, semantic lookup and generated reference. No report execution, composition engine or new dependencies.
