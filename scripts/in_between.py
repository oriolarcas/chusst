def index_to_rank_and_file(index):
    rank = index // 8
    file = index % 8
    return rank, file

print("const IN_BETWEEN_TABLE: [[u64; 64]; 64] = [")

for source_index in range(64):
    print("    [")
    for target_index in range(64):
        # Check if the source and target are on the same rank, file or diagonal
        source_rank, source_file = index_to_rank_and_file(source_index)
        target_rank, target_file = index_to_rank_and_file(target_index)
        if source_rank == target_rank or source_file == target_file or abs(source_rank - target_rank) == abs(source_file - target_file):
            # They are on the same rank, file or diagonal
            if source_index == target_index:
                # They are the same square
                print("        0,")
            else:
                # They are on the same rank, file or diagonal, but not the same square
                # Print the squares in between
                squares_in_between = []
                if source_rank == target_rank:
                    # They are on the same rank
                    for file in range(min(source_file, target_file) + 1, max(source_file, target_file)):
                        squares_in_between.append(source_rank * 8 + file)
                elif source_file == target_file:
                    # They are on the same file
                    for rank in range(min(source_rank, target_rank) + 1, max(source_rank, target_rank)):
                        squares_in_between.append(rank * 8 + source_file)
                else:
                    # They are on the same diagonal
                    for i in range(1, abs(source_rank - target_rank)):
                        if source_rank < target_rank:
                            rank = source_rank + i
                        else:
                            rank = source_rank - i
                        if source_file < target_file:
                            file = source_file + i
                        else:
                            file = source_file - i
                        squares_in_between.append(rank * 8 + file)
                if len(squares_in_between) == 0:
                    # There are no squares in between
                    print("        0,")
                else:
                    # Print the squares in between
                    print(f"        0x{sum([2 ** square for square in squares_in_between]):016x},")
        else:
            # They are not on the same rank, file or diagonal
            print("        0,")
    print("    ],")

print("];")
