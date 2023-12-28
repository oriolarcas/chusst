def index_to_rank_and_file(index):
    rank = index // 8
    file = index % 8
    return rank, file

def rank_and_file_to_index(rank, file):
    return rank * 8 + file

def bitboard_from_bit(index):
    return 1 << index

# A pawn can never be below its starting rank (1 for white or 8 for black)
# Also when it arrives to the promotion rank, it promotes and cannot be a pawn anymore
# So no possible moves for ranks 1 and 8

print("pub const WHITE_PAWN_ATTACK_TABLE: [u64; 64] = [")

for source_index in range(64):
    source_rank, source_file = index_to_rank_and_file(source_index)

    # Possible captures, not including en passant

    if source_rank == 0 or source_rank == 7:
        print("    0,")
        continue

    bitboard = 0
    if source_file > 0:
        target_index = rank_and_file_to_index(source_rank + 1, source_file - 1)
        bitboard |= bitboard_from_bit(target_index)

    if source_file < 7:
        target_index = rank_and_file_to_index(source_rank + 1, source_file + 1)
        bitboard |= bitboard_from_bit(target_index)

    print(f"    0x{bitboard:016x},")

print("];")

print()

print("pub const BLACK_PAWN_ATTACK_TABLE: [u64; 64] = [")

for source_index in range(64):
    source_rank, source_file = index_to_rank_and_file(source_index)

    # Possible captures, not including en passant

    if source_rank == 0 or source_rank == 7:
        print("    0,")
        continue

    bitboard = 0
    if source_file > 0:
        target_index = rank_and_file_to_index(source_rank - 1, source_file - 1)
        bitboard |= bitboard_from_bit(target_index)

    if source_file < 7:
        target_index = rank_and_file_to_index(source_rank - 1, source_file + 1)
        bitboard |= bitboard_from_bit(target_index)

    print(f"    0x{bitboard:016x},")

print("];")

print("pub const WHITE_ATTACKED_BY_PAWN_TABLE: [u64; 64] = [")

for source_index in range(64):
    source_rank, source_file = index_to_rank_and_file(source_index)

    # Possible captures, not including en passant

    if source_rank == 7:
        print("    0,")
        continue

    bitboard = 0
    if source_file > 0:
        target_index = rank_and_file_to_index(source_rank + 1, source_file - 1)
        bitboard |= bitboard_from_bit(target_index)

    if source_file < 7:
        target_index = rank_and_file_to_index(source_rank + 1, source_file + 1)
        bitboard |= bitboard_from_bit(target_index)

    print(f"    0x{bitboard:016x},")

print("];")

print("pub const BLACK_ATTACKED_BY_PAWN_TABLE: [u64; 64] = [")

for source_index in range(64):
    source_rank, source_file = index_to_rank_and_file(source_index)

    # Possible captures, not including en passant

    if source_rank == 0:
        print("    0,")
        continue

    bitboard = 0
    if source_file > 0:
        target_index = rank_and_file_to_index(source_rank - 1, source_file - 1)
        bitboard |= bitboard_from_bit(target_index)

    if source_file < 7:
        target_index = rank_and_file_to_index(source_rank - 1, source_file + 1)
        bitboard |= bitboard_from_bit(target_index)

    print(f"    0x{bitboard:016x},")

print("];")