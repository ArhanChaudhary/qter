prev_phase_info := function()
    return rec(
        first_cycles := Immutable([
            U*L*B^-1*L*B^-1*U*R^-1*D*U^2*L^2*F^2,
        ]),
        second_cycle_order := 18,
        share_edge := true,
        share_corner := true,
    );
end;

SetPrintFormattingStatus("*stdout*", false);
U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);
cube := Group(U, L, F, R, B, D);
prev_phase_info := prev_phase_info();
edge_facelet_buf := 42;
corner_facelet_buf := 1;
edge_cubies := Blocks(cube, Orbit(cube, edge_facelet_buf));
corner_cubies := Blocks(cube, Orbit(cube, corner_facelet_buf));
edge_facelets := Immutable(Flat(edge_cubies));
corner_facelets := Immutable(Flat(corner_cubies));

Read("util.g");

ValidCornerFlip := function(corner1, corner2)
    if corner1 * corner2 in cube then
        return corner1 * corner2;
    else
        return corner1 * corner2^-1;
    fi;
end;

# TODO: try longer phase 1 cycles from phase 2, they might result in a shorter
# phase 3 cycle

# ALL WAYS TO ARRANGE THE REST OF N - 1 cycles
# ADDENDUM: no because we care about the shortest algorithm for the most order
# cycle, symmetry when all cycle orders are the same is handled in phase 1
# ADDENDUM2: DONT DO THIS IN PHASE 1 DO IT HERE

for i in [1..Length(prev_phase_info.first_cycles)] do
    # first_cycle_moved_points:=Unique(Concatenation(first_cycle_moved_points, MovedPoints(tmp)));
    first_cycle := prev_phase_info.first_cycles[i];
    # Given our second cycle algorithm, we want to find the third
    first_cycle_moved_points := MovedPoints(first_cycle);
    first_cycle_stabilizer := Stabilizer(
        cube,
        first_cycle_moved_points,
        OnTuples
    );
    first_cycle_components := Cycles(first_cycle, first_cycle_moved_points);
    first_cycle_stabilizer_edge_cubies := Blocks(
        first_cycle_stabilizer,
        Difference(edge_facelets, first_cycle_moved_points)
    );
    first_cycle_stabilizer_corner_cubies := Blocks(
        first_cycle_stabilizer,
        Difference(corner_facelets, first_cycle_moved_points)
    );

    # List(first_cycle_stabilizer_corner_cubies, c -> ValidCornerFlip(CycleFromList(c), shared_corner_cubie));

    if prev_phase_info.share_edge or prev_phase_info.share_corner then
        shared_cubies := [];
        if prev_phase_info.share_edge then
            shared_edge_cubie := CycleFromList(First(
                first_cycle_components,
                c -> Length(c) = 2
            ));
            for second_edge_cubie in first_cycle_stabilizer_edge_cubies do
                Add(
                    shared_cubies,
                    CycleFromList(second_edge_cubie) * shared_edge_cubie
                );
            od;
        fi;
        if prev_phase_info.share_corner then
            # TODO: multiple shared corner cubies, use ALL previous unshared cubies
            shared_corner_cubie := CycleFromList(First(
                first_cycle_components,
                c -> Length(c) = 3
            ));
            for second_corner_cubie in first_cycle_stabilizer_corner_cubies do
                Add(
                    shared_cubies,
                    ValidCornerFlip(
                        CycleFromList(second_corner_cubie),
                        shared_corner_cubie
                    )
                );
            od;
        fi;
        second_cycle_group := ClosureGroup(
            first_cycle_stabilizer,
            shared_cubies
        );
    else
        second_cycle_group := first_cycle_stabilizer;
    fi;

    second_cycle_classes := ConjugacyClassesOfOrder(
        second_cycle_group,
        prev_phase_info.second_cycle_order
    );
    # second_cycle_classes := ConjugacyClassesOfStructure(second_cycle_group, [ 1,,,,,,,,1 ]);
    # TODO: FORCE FLIP SHARED CUBIE, FILTER CLASSES
    unique_length := 0;
    for j in [1..Length(second_cycle_classes)] do
        is_duplicate := false;
        inversed_class_element := Representative(second_cycle_classes[j])^-1;
        for k in [1..unique_length] do
            if inversed_class_element in second_cycle_classes[k] then
                is_duplicate := true;
                break;
            fi;
        od;
        if is_duplicate then
            continue;
        fi;
        unique_length := unique_length + 1;
        second_cycle_classes[unique_length] := second_cycle_classes[j];
        prune_inverse_elements := (
            inversed_class_element <> inversed_class_element^-1
            and inversed_class_element in second_cycle_classes[j]
        );
        for second_cycle_candidate in second_cycle_classes[j] do
            # thing := Cycles(second_cycle_candidate, MovedPoints(second_cycle_candidate));
            if (
                not prune_inverse_elements or
                second_cycle_candidate < second_cycle_candidate^-1
            # ) and [7, 18] in thing and ([1, 35, 9] in thing or [1, 9, 35] in thing) then
            ) then
                AppendTo(
                    "./output2.txt",
                    PermutationSpeffz(second_cycle_candidate),
                    "\n"
                );
            fi;
        od;
    od;
    # TODO: for every thing the same htm and qtm size. have a stack of shared cubies each cycle combination uses
od;
