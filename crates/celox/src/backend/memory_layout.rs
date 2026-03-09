use crate::ir::{AbsoluteAddr, Program};
use crate::{HashMap, SimulatorOptions};

#[derive(Debug, Clone)]
pub struct MemoryLayout {
    /// Stable region (region = 0) offsets. Includes all declared variables.
    pub offsets: HashMap<AbsoluteAddr, usize>,
    pub widths: HashMap<AbsoluteAddr, usize>,
    /// Whether the variable is a 4-state type.
    pub is_4states: HashMap<AbsoluteAddr, bool>,
    /// Stable region size in bytes.
    pub total_size: usize,

    /// Working region (region != 0) offsets. Includes only actually-used variables.
    pub working_offsets: HashMap<AbsoluteAddr, usize>,
    /// Base offset (bytes) of the working region inside the unified memory buffer.
    pub working_base_offset: usize,
    /// Unified memory buffer size in bytes (stable + working).
    pub merged_total_size: usize,

    /// Bitset of triggered domain IDs.
    pub triggered_bits_offset: usize,
    pub triggered_bits_total_size: usize,

    /// Scratch region for spilling inter-chunk registers.
    /// Located after triggered bits. Zero if no spilling needed.
    pub scratch_base_offset: usize,
    pub scratch_size: usize,

    /// Byte stride between consecutive array elements for decomposed arrays.
    /// Keyed by the blob address (element_index = None). Stride may exceed
    /// `get_byte_size(element_width)` due to alignment padding.
    pub element_strides: HashMap<AbsoluteAddr, usize>,
}

impl MemoryLayout {
    pub fn build(program: &Program, options: &SimulatorOptions) -> Self {
        let scratch_bytes = match &program.eval_comb_plan {
            Some(crate::ir::EvalCombPlan::MemorySpilled(plan)) => plan.scratch_bytes,
            _ => 0,
        };

        // Separate variables into scalars/non-decomposable blobs vs decomposable arrays.
        let mut scalar_vars = Vec::new();
        // (blob_addr, element_width_bits, total_elements, is_4state)
        let mut decomposable_arrays: Vec<(AbsoluteAddr, usize, usize, bool)> = Vec::new();

        for (instance_id, module_id) in &program.instance_module {
            let variables = &program.module_variables[module_id];
            for info in variables.values() {
                let addr = AbsoluteAddr {
                    instance_id: *instance_id,
                    var_id: info.id,
                    element_index: None,
                };
                let total_elements: usize = info.array_dims.iter().product();
                let element_width = if total_elements > 1 {
                    info.width / total_elements
                } else {
                    0
                };
                if total_elements > 1 && element_width % 8 == 0 {
                    decomposable_arrays.push((addr, element_width, total_elements, info.is_4state));
                } else {
                    scalar_vars.push((addr, info.width, info.is_4state));
                }
            }
        }

        scalar_vars.sort_by_key(|&(_, width, _)| std::cmp::Reverse(get_alignment(width)));
        // Sort decomposable arrays by element alignment (descending) for efficient packing.
        decomposable_arrays.sort_by_key(|&(_, ew, _, _)| std::cmp::Reverse(get_alignment(ew)));

        let mut offsets = HashMap::default();
        let mut widths = HashMap::default();
        let mut is_4states = HashMap::default();
        let mut element_strides = HashMap::default();
        let mut current_offset = 0;

        // Allocate scalar / non-decomposable variables as blobs.
        for (addr, width, is_4state) in scalar_vars {
            let align = get_alignment(width);
            let size = get_byte_size(width);
            current_offset = (current_offset + align - 1) & !(align - 1);

            offsets.insert(addr, current_offset);
            widths.insert(addr, width);
            is_4states.insert(addr, is_4state);

            current_offset += size;
            if options.four_state {
                current_offset += size;
            }
        }

        // Allocate decomposable array elements individually with alignment.
        for (blob_addr, element_width, total_elements, is_4state) in &decomposable_arrays {
            let align = get_alignment(*element_width);
            let elem_byte_size = get_byte_size(*element_width);
            let stride = (elem_byte_size + align - 1) & !(align - 1);

            // Align base offset for the first element.
            current_offset = (current_offset + align - 1) & !(align - 1);
            let base = current_offset;

            for i in 0..*total_elements {
                let elem_addr = AbsoluteAddr {
                    instance_id: blob_addr.instance_id,
                    var_id: blob_addr.var_id,
                    element_index: Some(i as u32),
                };
                offsets.insert(elem_addr, base + i * stride);
                widths.insert(elem_addr, *element_width);
                is_4states.insert(elem_addr, *is_4state);
            }

            // Blob entry points to element[0] for API / NAPI backward compatibility.
            offsets.insert(*blob_addr, base);
            widths.insert(*blob_addr, *element_width * *total_elements);
            is_4states.insert(*blob_addr, *is_4state);
            element_strides.insert(*blob_addr, stride);

            current_offset = base + *total_elements * stride;
            if options.four_state {
                current_offset += *total_elements * stride;
            }
        }

        // Compact working region: only variables actually written in WORKING region.
        let working_addrs = program.collect_working_region_addrs();
        let mut working_vars_to_layout: Vec<(AbsoluteAddr, usize, bool)> = working_addrs
            .iter()
            .filter_map(|addr| {
                if let Some(&w) = widths.get(addr) {
                    Some((*addr, w, is_4states[addr]))
                } else if addr.element_index.is_some() {
                    // Derive from the blob entry.
                    let blob_addr = AbsoluteAddr {
                        element_index: None,
                        ..*addr
                    };
                    if let Some(&is_4s) = is_4states.get(&blob_addr) {
                        if let Some((_, ew, _, _)) = decomposable_arrays
                            .iter()
                            .find(|(a, _, _, _)| *a == blob_addr)
                        {
                            Some((*addr, *ew, is_4s))
                        } else if let Some(&total_w) = widths.get(&blob_addr) {
                            Some((*addr, total_w, is_4s))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        working_vars_to_layout
            .sort_by_key(|&(_, width, _)| std::cmp::Reverse(get_alignment(width)));

        let mut working_offsets = HashMap::default();
        let mut working_current_offset = 0;
        for (addr, width, _is_4state) in working_vars_to_layout {
            let align = get_alignment(width);
            let size = get_byte_size(width);
            working_current_offset = (working_current_offset + align - 1) & !(align - 1);

            working_offsets.insert(addr, working_current_offset);

            working_current_offset += size;
            if options.four_state {
                working_current_offset += size;
            }
        }

        // Keep working region properly aligned when appended to the stable region.
        let working_base_offset = (current_offset + 7) & !7;

        // Triggered bits region (1 bit per event canonical ID)
        let num_potential_triggers = program.num_events();
        let triggered_bits_offset = (working_base_offset + working_current_offset + 7) & !7;
        let triggered_bits_total_size = num_potential_triggers.div_ceil(8);

        let scratch_base_offset = (triggered_bits_offset + triggered_bits_total_size + 7) & !7;
        let merged_total_size = (scratch_base_offset + scratch_bytes + 7) & !7;

        Self {
            offsets,
            widths,
            is_4states,
            total_size: current_offset,
            working_offsets,
            working_base_offset,
            merged_total_size,
            triggered_bits_offset,
            triggered_bits_total_size,
            scratch_base_offset,
            scratch_size: scratch_bytes,
            element_strides,
        }
    }
}

pub fn get_byte_size(width: usize) -> usize {
    (width + 7) >> 3
}

fn get_alignment(width: usize) -> usize {
    let size = get_byte_size(width);
    if size == 0 {
        1
    } else if size <= 8 {
        size.next_power_of_two()
    } else {
        8
    }
}
