
#[derive(Debug)]
pub struct SyscallPatchHook<'a> {
    // NB: if the patched sequence contains multiple
    // instructions, it is possible in the same function
    // there is a jmp @label within the very function,
    // and the @label is within the range of the patched
    // multiple instructions. This could cause the function
    // jumps to the middle of our patched sequence, which is
    // likely cause undefined behavior.
    // one example is `clock_nanosleep` in glibc.
    pub is_multi: bool,
    pub instructions: &'a [u8],
    pub symbol: &'a str,
}

pub static SYSCALL_HOOKS: &'static [SyscallPatchHook] = &[
    /* Many glibc syscall wrappers (e.g. read) have 'syscall' followed by
     * cmp $-4095,%rax */
    SyscallPatchHook {
        is_multi: false,
        instructions: &[0x48, 0x3d, 0x01, 0xf0, 0xff, 0xff],
        symbol: "_syscall_hook_trampoline_48_3d_01_f0_ff_ff",
    },
    /* Many glibc syscall wrappers (e.g. __libc_recv) have 'syscall'
     * followed by
     * cmp $-4096,%rax */
    SyscallPatchHook {
        is_multi: false,
        instructions: &[0x48, 0x3d, 0x00, 0xf0, 0xff, 0xff],
        symbol: "_syscall_hook_trampoline_48_3d_00_f0_ff_ff",
    },
    /* Many glibc syscall wrappers (e.g. read) have 'syscall' followed by
     * mov (%rsp),%rdi */
    SyscallPatchHook {
        is_multi: false,
        instructions: &[0x48, 0x8b, 0x3c, 0x24],
        symbol: "_syscall_hook_trampoline_48_8b_3c_24",
    },
    /* __lll_unlock_wake has 'syscall' followed by
     * pop %rdx; pop %rsi; ret */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[0x5a, 0x5e, 0xc3], 
        symbol: "_syscall_hook_trampoline_5a_5e_c3",
    },
    /* posix_fadvise64 has 'syscall' followed by
     * mov %eax,%edx;
     * neg %edx */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[ 0x89, 0xc2, 0xf7, 0xda ],
        symbol: "_syscall_hook_trampoline_89_c2_f7_da",
    },
    /* Our VDSO vsyscall patches have 'syscall' followed by
     * nop; nop; nop */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[ 0x90, 0x90, 0x90 ],
        symbol: "_syscall_hook_trampoline_90_90_90",
    },
    /* glibc-2.22-17.fc23.x86_64 has 'syscall' followed by 
     * 'mov $1,%rdx' in pthread_barrier_wait.
     */
    SyscallPatchHook {
        is_multi: false,
        instructions: &[ 0xba, 0x01, 0x00, 0x00, 0x00 ],
        symbol: "_syscall_hook_trampoline_ba_01_00_00_00",
    },
    /* pthread_sigmask has 'syscall' followed by 
     * 'mov %eax,%ecx;
     *  xor %edx,%edx' */
    SyscallPatchHook { 
        is_multi: true,
        instructions: &[ 0x89, 0xc1, 0x31, 0xd2 ],
        symbol: "_syscall_hook_trampoline_89_c1_31_d2",
    },
    /* getpid has 'syscall' followed by
     * 'retq;
     *  nopl 0x0(%rax,%rax,1) */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[ 0xc3, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00 ],
        symbol: "_syscall_hook_trampoline_c3_nop",
    },
    /* liblsan internal_close has 'syscall' followed by
     * 'retq;
     *  nopl 0x0(%rax,%rax,1) */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[ 0xc3, 0x0f, 0x1f, 0x44, 0x00, 0x00 ],
        symbol: "_syscall_hook_trampoline_c3_nop",
    },
    /* liblsan internal_open has 'syscall' followed by
     * 'retq;
     *  nopl (%rax) */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[ 0xc3, 0x0f, 0x1f, 0x00 ],
        symbol: "_syscall_hook_trampoline_c3_nop",
    },
    /* liblsan internal_dup2 has 'syscall' followed by
     * 'retq;
     *  xchg %ax,%ax */
    SyscallPatchHook {
        is_multi: true,
        instructions: &[ 0xc3, 0x66, 0x90 ],
        symbol: "_syscall_hook_trampoline_c3_nop",
    },
];