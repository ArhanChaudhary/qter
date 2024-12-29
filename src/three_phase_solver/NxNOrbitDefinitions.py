
def NxNDefinitions(N):

    definition_text = 'PUZZLE_$N$x$N$ = PuzzleOrbitDefinition(\n\torbits=[\n$orbits$\t],\n\teven_parity_constraints=(\n$parities$\t),\n)'
    orbit_text = '\t\tOrbit(\n\t\t\tname=$name$,\n\t\t\tcubie_count=$count$,\n\t\t\torientation_status=OrientationStatus.$status$\n\t\t),\n'
    can_orient = 'CanOrient(\n\t\t\t\tcount=$count$,\n\t\t\t\tsum_constraint=OrientationSumConstraint.$constraint$,\n\t\t\t),'
    cannot_orient = 'CannotOrient(),'
    even_parity_text = '\t\tEvenParityConstraint(\n\t\t\torbit_names=($names$),\n\t\t),\n'

    orbits = ''
    center_begin = 1
    if N % 2 == 1:
        center_begin = 0
        orbits += orbit_text.replace('$name$','"edges"').replace('$count$','12').replace('$status$',can_orient.replace('$count$','2').replace('$constraint$','ZERO'))


    orbits += orbit_text.replace('$name$','"corners"').replace('$count$','8').replace('$status$',can_orient.replace('$count$','3').replace('$constraint$','ZERO'))

    for w in range(1,N//2):
        orbits += orbit_text.replace('$name$','"wings'+str(w)+'"').replace('$count$','24').replace('$status$',cannot_orient)
    for c1 in range(center_begin,N//2):
        for c2 in range(1,N//2):
            orbits += orbit_text.replace('$name$','"centers'+str(c1)+';'+str(c2)+'"').replace('$count$','24').replace('$status$',cannot_orient)

    
    parities = ''
    if N % 2 == 1:
        parities += even_parity_text.replace('$names$','"corners", "edges"')
        for c2 in range(1,N//2):
            parities += even_parity_text.replace('$names$','"corners", "wings'+str(c2)+'", "centers0;'+str(c2)+'"')

    for c1 in range(1,N//2):
        parities += even_parity_text.replace('$names$','"corners", "centers'+str(c1)+';'+str(c1)+'"')
        for c2 in range(1,N//2):
            if c1 == c2:
                continue
            parities += even_parity_text.replace('$names$', '"corners", "wings'+str(c1)+'", "wings'+str(c2)+'", "centers'+str(c1)+';'+str(c2)+'"')

    return definition_text.replace('$orbits$',orbits).replace('$parities$',parities).replace('$N$',str(N))

print(NxNDefinitions(6))

