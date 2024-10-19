SetPrintFormattingStatus("*stdout*", false);

U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);;
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);;
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);;
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);;
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);;
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);;
first_cycles := [
    U*L*F*L^-1*R^2,
    U*F*R*F^-1*B^2,
    U*R*B*R^-1*L^2,
    U*B*L*B^-1*F^2,
    U*L*U^-1*D^2*F,
    U*F*U^-1*D^2*R,
    U*R*U^-1*D^2*B,
    U*B*U^-1*D^2*L,
    U*L^-1*R^2*B*L,
    U*F^-1*B^2*L*F,
    U*R^-1*L^2*F*R,
    U*B^-1*F^2*R*B,
    U*L^-1*U^-1*F^-1*D^2,
    U*F^-1*U^-1*R^-1*D^2,
    U*R^-1*U^-1*B^-1*D^2,
    U*B^-1*U^-1*L^-1*D^2,
    U*D^2*L^-1*U^-1*F^-1,
    U*D^2*F^-1*U^-1*R^-1,
    U*D^2*R^-1*U^-1*B^-1,
    U*D^2*B^-1*U^-1*L^-1
];
second_cycle_orders := [
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24,
    24
];

cube := Group(U, L, F, R, B, D);;
edges := [
    [2, 34],  [5, 26],  [7, 18],  [4, 10], [13, 20], [21, 28],
    [29, 36], [37, 12], [42, 23], [45, 31], [47, 39], [44, 15]
];
corners := [
    [1, 9, 35], [3, 33, 27], [8, 25, 19], [6, 17, 11],
    [41, 16, 22], [43, 24, 30], [48, 32, 38], [46, 40, 14]
];
flat_edges := Flat(edges);
flat_corners := Flat(corners);
top_face := [1..8];
bottom_face := [41..48];
edge_buf := 42;
corner_buf := 1;
facelet_to_speffz := "aabdbdcceefhfhggiijljlkkmmnpnpooqqrtrtssuuvxvxww";

PermutationSpeffz := function(perm)
    local cycles, edge_cycles, corner_cycles, ret, corners_or_edges, buf, i,
        cycle, cycle_facelet, visited_facelets, buf_pos, orient_facelet_index,
        before_orient_facelet_index, starting_cubie;
    cycles := Filtered(Cycles(perm, [1..48]), c -> Length(c) <> 1);
    edge_cycles := Filtered(cycles, c -> ForAny(c, f -> f in flat_edges));
    corner_cycles := Filtered(cycles, c -> ForAny(c, f -> f in flat_corners));
    ret := "";
    for corners_then_edges in [1..2] do
        if corners_then_edges = 1 then
            corners_or_edges := corners;
            cycles := corner_cycles;
            buf := 1;
        else
            Add(ret, '.');
            corners_or_edges := edges;
            cycles := edge_cycles;
            buf := 42;
        fi;
        cycle := First(cycles, c -> buf in c);
        if cycle = fail then
            visited_facelets := [];
        else
            buf_pos := PositionProperty(cycle, c -> c = buf);
            visited_facelets := ShallowCopy(First(
                corners_or_edges,
                c -> buf in c
            ));
            i := 1;
            while true do
                cycle_facelet := cycle[
                    (buf_pos + i - 1) mod Length(cycle) + 1
                ];
                if cycle_facelet in visited_facelets then
                    break;
                fi;
                Add(ret, facelet_to_speffz[cycle_facelet]);
                i := i + 1;
            od;
        fi;
        for cycle in cycles do
            if ForAny(cycle, c -> Position(visited_facelets, c) <> fail) then
                continue;
            fi;
            starting_cubie := ShallowCopy(First(
                corners_or_edges,
                c -> cycle[1] in c
            ));
            if ForAny(corners_or_edges, c -> IsEqualSet(c, cycle)) then
                if corners_then_edges = 1 then
                    orient_facelet_index := PositionProperty(cycle,
                        c -> c in top_face or c in bottom_face
                    );
                    before_orient_facelet_index := (
                        orient_facelet_index - 2
                    ) mod 3 + 1;
                else
                    before_orient_facelet_index := 1;
                fi;
                Add(ret, UppercaseChar(facelet_to_speffz[cycle[
                    before_orient_facelet_index
                ]]));
            else
                Add(ret, facelet_to_speffz[cycle[1]]);
                i := 2;
                while true do
                    cycle_facelet := cycle[(i - 1) mod Length(cycle) + 1];
                    Add(ret, facelet_to_speffz[cycle_facelet]);
                    if cycle_facelet in starting_cubie then
                        break;
                    fi;
                    i := i + 1;
                od;
            fi;
            Append(visited_facelets, starting_cubie);
        od;
    od;
    return ret;
end;

for i in [1..Length(first_cycles)] do
    first_cycle := first_cycles[i];
    first_cycle_moved_points := MovedPoints(first_cycle);
    first_cycle_stabilizer := Stabilizer(
        cube,
        first_cycle_moved_points,
        OnTuples
    );
    first_cycle_components := Cycles(first_cycle, first_cycle_moved_points);
    shared_edge := CycleFromList(First(
        first_cycle_components,
        c -> Length(c) = 2
    ));
    shared_corner := CycleFromList(First(
        first_cycle_components,
        c -> Length(c) = 3
    ));
    random_second_cycle_edge := CycleFromList(First(
        edges,
        e -> not e[1] in first_cycle_moved_points and
        not e[2] in first_cycle_moved_points
    ));
    random_second_cycle_corner := CycleFromList(First(
        corners,
        c -> not c[1] in first_cycle_moved_points and
        not c[2] in first_cycle_moved_points
    ));
    second_cycle_group := Group(Concatenation(
        GeneratorsOfGroup(first_cycle_stabilizer),
        [
            # im not sure why, but this probably generates everything
            shared_edge * random_second_cycle_edge,
            shared_corner * random_second_cycle_corner,
        ]
    ));
    second_cycle_classes := Filtered(
        ConjugacyClasses(second_cycle_group),
        c -> Order(Representative(c)) = second_cycle_orders[i]
    );
    AppendTo("./output2.txt", "\nIndex ", i, ": \n");
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
        for second_cycle_candidate in second_cycle_classes[j] do
            AppendTo(
                "./output2.txt",
                PermutationSpeffz(second_cycle_candidate),
                "\n"
            );
        od;
    od;
od;
