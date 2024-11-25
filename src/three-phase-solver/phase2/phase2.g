LoadPackage("datastructures");

U := ( 1, 3, 8, 6)( 2, 5, 7, 4)( 9,33,25,17)(10,34,26,18)(11,35,27,19);
L := ( 9,11,16,14)(10,13,15,12)( 1,17,41,40)( 4,20,44,37)( 6,22,46,35);
F := (17,19,24,22)(18,21,23,20)( 6,25,43,16)( 7,28,42,13)( 8,30,41,11);
R := (25,27,32,30)(26,29,31,28)( 3,38,43,19)( 5,36,45,21)( 8,33,48,24);
B := (33,35,40,38)(34,37,39,36)( 3, 9,46,32)( 2,12,47,29)( 1,14,48,27);
D := (41,43,48,46)(42,45,47,44)(14,22,30,38)(15,23,31,39)(16,24,32,40);
cube := Group(U, L, F, R, B, D);

Read("../phase3/util.g");

cornercube := ClosureGroup(
    Stabilizer(cube, Orbit(cube, edge_facelet_buf), OnTuples),
    (3, 8)(19, 27)(25, 33)
);
cornercube_enumerator := Enumerator(cornercube);
moves := [U, L, F, R, B, D, U^-1, L^-1, F^-1, R^-1, B^-1, D^-1, U^2, L^2, F^2, R^2, B^2, D^2];;
len_moves := Length(moves);

HashPerm := function(perm)
    # TODO: use lehmer code + ternary instead
    return Position(cornercube_enumerator, perm);
end;

BFSFromStructure := function(target_cycle_structure)
    local queue, perm, curr_depth, new_perm, j, i, distances, class,
        classes, queue_length, k, hash_perm, cache, visited_count;
    Print("** Finding conjugacy classes **\n");
    classes := ConjugacyClassesOfStructure(cornercube, target_cycle_structure);
    Print("** Initializing search **\n");
    distances := ListWithIdenticalEntries(Size(cornercube), -1);
    queue := PlistDeque();
    for class in classes do
        for perm in class do
            distances[HashPerm(perm)] := 0;
            PlistDequePushBack(queue, perm);
        od;
    od;
    Unbind(class);
    Unbind(classes);
    Print("** Running BFS **\n");
    curr_depth := 0;
    visited_count := 0;
    while not IsEmpty(queue) do
        queue_length := Size(queue);
        curr_depth := curr_depth + 1;
        Print("** Populating BFS depth ", curr_depth, " exploring ", queue_length, " nodes (", Int(visited_count / Size(cornercube) * 100), "%) done **\n");
        for i in [1..queue_length] do
            perm := PlistDequePopFront(queue);
            for j in [1..12] do
                new_perm := ListPerm(perm * moves[j]);
                for k in edge_facelets do
                    if k <= Length(new_perm) then
                        new_perm[k] := k;
                    fi;
                od;
                new_perm := PermList(new_perm);
                hash_perm := HashPerm(new_perm);
                if distances[hash_perm] = -1 or curr_depth < distances[hash_perm] then
                    if distances[hash_perm] = -1 then
                        visited_count := visited_count + 1;
                    fi;
                    distances[hash_perm] := curr_depth;
                    PlistDequePushBack(queue, new_perm);
                fi;
            od;
        od;
    od;
    return distances;
end;

start := Runtime();
a:=BFSFromStructure([ ,1,,,,,,1 ]);;
AppendTo("out.txt", "Time taken: ", Runtime() - start, "ms");
SaveWorkspace("save");