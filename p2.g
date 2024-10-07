U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);;
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);;
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);;
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);;
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);;
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);;

first_cycle := U*L*F*L^-1*R^2;
second_cycle_order := 24; # or the cycle structure

cube := Group(U, L, F, R, B, D);;
generator_names := ["U", "L", "F", "R", "B", "D"];
hom := EpimorphismFromFreeGroup(cube:names:=generator_names);;
first_cycle_stabilizer := Stabilizer(cube, MovedPoints(first_cycle), OnTuples);
first_cycle_components := Cycles(first_cycle, MovedPoints(first_cycle));
shared_edge := CycleFromList(First(first_cycle_components, c -> Length(c) = 2));
shared_corner := CycleFromList(First(first_cycle_components, c -> Length(c) = 3));
second_cycle_group := Group(Concatenation(GeneratorsOfGroup(first_cycle_stabilizer), [shared_edge, shared_corner]));
second_cycle_candidates := Filtered(second_cycle_group, k -> Order(k) = second_cycle_order);

SetPrintFormattingStatus("*stdout*", false);
for second_cycle_candidate in second_cycle_candidates do
    ext_rep := ExtRepOfObj(PreImagesRepresentative(hom, second_cycle_candidate));
    for i in [1..Length(ext_rep) / 2] do
        if i <> 1 then
            Print(" ");
        fi;
        move := generator_names[ext_rep[i * 2 - 1]];
        Print(move);
        count := ext_rep[i * 2];
        if count in [-2, 2] then
            Print("2");
        elif count in [-1, 3] then
            Print("'");
        fi;
    od;
    Print("\n");
od;
