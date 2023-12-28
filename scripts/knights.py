def index_to_rank_and_file(index):
    rank = index // 8
    file = index % 8
    return rank, file

def rank_and_file_to_index(rank, file):
    return rank * 8 + file

def bitboard_from_bit(index):
    return 1 << index

def is_valid_position(rank, file):
    return rank >= 0 and rank < 8 and file >= 0 and file < 8

def format_bitboard(bitboard):
    print("  abcdefgh")
    for rank_number, rank_byte in reversed(list(enumerate(bitboard.to_bytes(8, 'little')))):
        print(f"{rank_number + 1} {rank_byte:08b}")

print("pub const KNIGHT_ATTACK_TABLE: [u64; 64] = [")

for source_index in range(64):
    source_rank, source_file = index_to_rank_and_file(source_index)

    moves = [
        (-1, -2),
        (-1, 2),
        (-2, -1),
        (-2, 1),
        (2, -1),
        (2, 1),
        (1, -2),
        (1, 2),
    ]

    bitboard = 0

    for move_ranks, move_files in moves:
        target_rank = source_rank + move_ranks
        target_file = source_file + move_files

        if not is_valid_position(target_rank, target_file):
            continue

        target_index = rank_and_file_to_index(target_rank, target_file)
        bitboard |= bitboard_from_bit(target_index)

    if bitboard == 0:
        print("    0,")
    else:
        print(f"    0x{bitboard:016x},")
        # print(f"{format_bitboard(bitboard | bitboard_from_bit(source_index))}")


print("];")
