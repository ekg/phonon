#!/bin/bash
# Generate complete node inventory with parameter counts

echo "# Phonon Node Inventory"
echo "Generated: $(date)"
echo ""
echo "| Node | Parameters | File |"
echo "|------|------------|------|"

for node_file in src/nodes/*.rs; do
    node_name=$(basename "$node_file" .rs)

    # Count NodeId parameters (input parameters for AudioNodes)
    node_id_count=$(grep -o "NodeId" "$node_file" | wc -l)

    echo "| $node_name | $node_id_count | $node_file |"
done | sort

echo ""
echo "## Summary"
echo "Total node files: $(find src/nodes -name '*.rs' | wc -l)"
echo "Total structs: $(grep -h 'pub struct.*Node' src/nodes/*.rs | wc -l)"
