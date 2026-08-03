# ADR-00XX: GPU Acceleration Integration

## Status
**Proposed**  
*Author: devstral (Mistral Vibe)*  
*Date: 2026-08-03*  
*Supersedes: N/A*

## Context

### Current State
- SynthPass is **CPU-only** by design (per `SYNTHPASS_ENGINEERING_CONSTITUTION.md` §4.1)
- OCR backend: `rten` (Rust Tensor Engine) + `ocrs` crate
- LLM backend: `llama-cpp-sys` (llama.cpp via Rust FFI)
- Performance budgets: OCR < 700ms, LLM < 2s, End-to-end < 3s (typical)
- M4 Tier-1 hit-rate: Currently **68%** (above 30% CI gate, 55% original target)

### Motivation
Customer request for GPU acceleration support to:
1. **Improve throughput** for batch processing (3-6x speedup expected)
2. **Reduce latency** for single-document extraction
3. **Leverage available hardware** (customer has GTX 970 with 3.5GB GDDR5)
4. **Maintain all existing guarantees**: correctness, security, determinism, offline-first

### Customer Hardware
- **GPU**: ZOTAC GTX 970
- **VRAM**: 3.5GB GDDR5
- **Compute Capability**: 5.2 (Maxwell architecture)
- **CUDA Support**: Yes (CUDA 11.x / 12.x)
- **Vulkan Support**: Yes
- **Memory**: Sufficient for OCR models (~550MB total) + small LLM (1.5GB in 4-bit)

---

## Decision

**Implement GPU acceleration as optional, compile-time feature flags** while maintaining CPU as the default and primary path.

### Core Principles
1. **CPU remains default** - No breaking changes to existing deployments
2. **Optional feature flags** - GPU support is opt-in via Cargo features
3. **Runtime fallback** - If GPU is unavailable, fall back to CPU gracefully
4. **Deterministic behavior** - Same input → same output regardless of backend
5. **Offline-first** - No external services, no data exfiltration
6. **Zero new runtime dependencies** - GPU libraries are system-level (CUDA, Vulkan drivers)

### Backend Selection Strategy
```
┌─────────────────────────────────────────────────────────────┐
│                    Backend Selection                          │
├─────────────────────────────────────────────────────────────┤
│  Compile-time Features                                        │
│  ├─ CPU (default): Always available                           │
│  ├─ CUDA: Enable with --features cuda                         │
│  └─ Vulkan: Enable with --features vulkan                     │
│                                                               │
│  Runtime Detection (if feature enabled):                       │
│  ├─ Check for GPU availability                                │
│  ├─ Check for compatible drivers                              │
│  └─ Fall back to CPU if unavailable                            │
└─────────────────────────────────────────────────────────────┘
```

### Feature Flag Structure
```toml
# Workspace-level features
[workspace]
members = [
    "crates/synthpass-ocr",
    "crates/synthpass-llm",
    # ...
]

# Per-crate features
[package]
# In synthpass-ocr/Cargo.toml
[features]
default = []
cuda = ["rten/cuda"]    # rten with CUDA support
vulkan = ["rten/vulkan"] # rten with Vulkan support

# In synthpass-llm/Cargo.toml  
[features]
default = []
cuda = ["llama-cpp-sys/cuda"]
vulkan = ["llama-cpp-sys/vulkan"]
metal = ["llama-cpp-sys/metal"]  # macOS only

# Convenience workspace feature
[workspace.metadata]
gpu = ["cuda", "vulkan"]
```

---

## Architecture

### Crate-Level Changes

#### 1. `synthpass-ocr` (OCR Backend)
**Changes:**
- Add `cuda` and `vulkan` feature flags
- Pass features to `rten` dependency
- Add `BackendConfig` enum for runtime selection
- Implement GPU availability detection
- Maintain CPU as default behavior

**Dependency updates:**
```toml
[dependencies]
rten = { version = "0.24.0", default-features = false }

[features]
cuda = ["rten/cuda"]
vulkan = ["rten/vulkan"]
```

**New types:**
```rust
/// OCR computation backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrBackend {
    /// CPU-only (default)
    Cpu,
    /// CUDA-accelerated
    Cuda,
    /// Vulkan-accelerated
    Vulkan,
}

/// Configuration for OCR backend
#[derive(Debug, Clone)]
pub struct OcrConfig {
    pub backend: OcrBackend,
    // Future: batch size, memory limits, etc.
}
```

**Runtime detection:**
```rust
impl OcrBackend {
    /// Auto-detect best available backend based on compile-time features
    pub fn auto() -> Self {
        // Try GPU backends if features enabled and hardware available
        #[cfg(feature = "cuda")]
        if Self::cuda_available() {
            return Self::Cuda;
        }
        
        #[cfg(feature = "vulkan")]
        if Self::vulkan_available() {
            return Self::Vulkan;
        }
        
        // Default to CPU
        Self::Cpu
    }
    
    /// Check CUDA availability at runtime
    #[cfg(feature = "cuda")]
    fn cuda_available() -> bool {
        // Use rten's CUDA detection
        rten::cuda::is_available()
    }
    
    /// Check Vulkan availability at runtime
    #[cfg(feature = "vulkan")]
    fn vulkan_available() -> bool {
        // Use rten's Vulkan detection
        rten::vulkan::is_available()
    }
}
```

#### 2. `synthpass-llm` (LLM Backend)
**Changes:**
- Add `cuda`, `vulkan`, `metal` feature flags
- Pass features to `llama-cpp-sys` dependency
- Add `LlmBackend` enum for runtime selection
- Implement GPU context initialization

**Dependency updates:**
```toml
[dependencies]
llama-cpp-sys = { version = "0.1.151", default-features = false }

[features]
cuda = ["llama-cpp-sys/cuda"]
vulkan = ["llama-cpp-sys/vulkan"]
metal = ["llama-cpp-sys/metal"]
```

**New types:**
```rust
/// LLM inference backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmBackend {
    /// CPU-only (default)
    Cpu,
    /// CUDA-accelerated
    Cuda,
    /// Vulkan-accelerated
    Vulkan,
    /// Metal-accelerated (macOS only)
    Metal,
}
```

#### 3. `synthpass-pipeline` (Orchestration)
**Changes:**
- Accept backend configuration
- Pass backend config to OCR and LLM providers
- Ensure deterministic behavior across backends

#### 4. `synthpass-cli` (CLI)
**Changes:**
- Add `--gpu-backend` flag (auto/cuda/vulkan/cpu)
- Add `--gpu-memory` flag for VRAM limits
- Show GPU info in `synthpass doctor`

**CLI changes:**
```rust
#[derive(Debug)]
struct GpuArgs {
    #[arg(long, value_enum, default_value = "auto")]
    backend: GpuBackendChoice,
    
    #[arg(long)]
    gpu_memory_mb: Option<usize>,
}

#[derive(ValueEnum, Clone, Debug)]
enum GpuBackendChoice {
    Auto,
    Cpu,
    Cuda,
    Vulkan,
}
```

#### 5. `synthpass-serve` (Web API)
**Changes:**
- Add environment variables for GPU configuration:
  - `SYNTHPASS_OCR_BACKEND=cuda|vulkan|cpu|auto`
  - `SYNTHPASS_LLM_BACKEND=cuda|vulkan|metal|cpu|auto`
  - `SYNTHPASS_GPU_MEMORY_MB=...`

#### 6. `synthpass-bench` (Benchmarking)
**Changes:**
- Add `--gpu-backend` flag for GPU-enabled benchmarks
- Add `--compare-backends` flag to compare CPU vs GPU
- Report GPU memory usage in benchmarks

---

## Implementation Phases

### Phase 1: Foundation (High Priority)
**Objective:** Core GPU support in OCR and LLM crates

| Task | Crate | Description | Estimated Effort |
|------|-------|-------------|-----------------|
| gpu_plan_001 | knowledge/decisions/ | ✅ Create this ADR document | Done |
| gpu_plan_003 | synthpass-ocr | Add CUDA/Vulkan features to Cargo.toml | 1 hour |
| gpu_plan_004 | synthpass-llm | Add CUDA/Vulkan/Metal features to Cargo.toml | 1 hour |
| gpu_plan_005 | synthpass-ocr | Implement backend selection in lib.rs | 2 hours |
| gpu_plan_006 | synthpass-llm | Implement backend selection in lib.rs | 2 hours |
| gpu_plan_007 | synthpass-ocr, synthpass-llm | Add runtime GPU detection | 2 hours |
| gpu_plan_015 | All | Run cargo fmt, clippy, test | 1 hour |

**Deliverables:**
- GPU features compile successfully
- CPU path unchanged and still works
- All existing tests pass

### Phase 2: Integration (High Priority)
**Objective:** Wire GPU support through the pipeline

| Task | Crate | Description | Estimated Effort |
|------|-------|-------------|-----------------|
| gpu_plan_002 | Cargo.toml (workspace) | Add workspace-level GPU features | 1 hour |
| gpu_plan_008 | synthpass-cli | Add --gpu-backend flag | 2 hours |
| gpu_plan_009 | synthpass-serve | Add GPU environment variables | 2 hours |
| gpu_plan_013 | synthpass-bench | Add GPU benchmark flags | 2 hours |
| gpu_plan_015 | All | Run cargo fmt, clippy, test | 1 hour |

**Deliverables:**
- CLI can select GPU backend
- API can be configured for GPU
- Benchmarks can measure GPU performance

### Phase 3: Testing & Documentation (Medium Priority)
**Objective:** Ensure quality and document the feature

| Task | Crate | Description | Estimated Effort |
|------|-------|-------------|-----------------|
| gpu_plan_010 | docker/ | Create GPU-enabled Docker images | 2 hours |
| gpu_plan_011 | .github/workflows/ | Add GPU CI test job | 2 hours |
| gpu_plan_012 | README.md, ARCHITECTURE.md | Update documentation | 2 hours |
| gpu_plan_014 | scripts/ | Create GPU benchmark comparison script | 2 hours |
| gpu_plan_015 | All | Run cargo fmt, clippy, test | 1 hour |

**Deliverables:**
- GPU CI passes
- Documentation updated
- Easy GPU testing for developers

---

## Performance Expectations

### GTX 970 (Maxwell, 3.5GB GDDR5)

| Component | CPU (Baseline) | CUDA (Expected) | Vulkan (Expected) | Speedup |
|-----------|---------------|-----------------|------------------|---------|
| OCR (rten) | ~6-8s/doc | ~1-2s/doc | ~1.5-2.5s/doc | **3-6x** |
| LLM (llama.cpp) | ~60-120s/doc | ~10-30s/doc | ~15-40s/doc | **2-6x** |
| End-to-end | ~15-20min (50 docs) | ~5-10min (50 docs) | ~7-12min (50 docs) | **2-3x** |

### Memory Usage

| Component | CPU | CUDA (GTX 970) |
|-----------|-----|----------------|
| OCR models | ~550MB RAM | ~550MB VRAM |
| LLM (Qwen 1.5B 4-bit) | ~1.5GB RAM | ~1.5GB VRAM |
| **Total** | ~2GB RAM | ~2GB VRAM (fits in 3.5GB) |

✅ **GTX 970 can handle both OCR + LLM simultaneously**

---

## Compatibility Matrix

### GPU Backends by Platform

| Platform | CUDA | Vulkan | Metal |
|----------|------|--------|-------|
| Linux | ✅ Yes | ✅ Yes | ❌ No |
| Windows | ✅ Yes | ✅ Yes | ❌ No |
| macOS | ❌ No | ✅ Yes | ✅ Yes |

### GPU Support by Hardware

| GPU | Compute Capability | CUDA | Vulkan | Notes |
|-----|-------------------|------|--------|-------|
| GTX 970 | 5.2 (Maxwell) | ✅ Yes | ✅ Yes | Customer hardware |
| GTX 10xx | 6.x (Pascal) | ✅ Yes | ✅ Yes | 
| RTX 20xx | 7.x (Turing) | ✅ Yes | ✅ Yes | 
| RTX 30xx | 8.x (Ampere) | ✅ Yes | ✅ Yes | 
| RTX 40xx | 9.x (Ada) | ✅ Yes | ✅ Yes | 
| Apple Silicon | N/A | ❌ No | ✅ Yes | Metal only |

### Minimum Requirements

**CUDA:**
- NVIDIA GPU with compute capability ≥ 5.0 (Kepler+)
- CUDA Toolkit 11.0+
- Driver: 450.80.02+

**Vulkan:**
- Any GPU with Vulkan 1.1+
- Driver with Vulkan support

---

## Build & Runtime Configuration

### Compile-time Features

```bash
# CPU-only (default) - no changes needed
cargo build --workspace

# CUDA-enabled build
cargo build --workspace --features cuda

# Vulkan-enabled build  
cargo build --workspace --features vulkan

# Both CUDA and Vulkan
cargo build --workspace --features "cuda,vulkan"

# All GPU features
cargo build --workspace --features "cuda,vulkan,metal"
```

### Runtime Configuration

**CLI:**
```bash
# Auto-detect best GPU backend
synthpass extract --image passport.png --gpu-backend auto

# Force CUDA
synthpass extract --image passport.png --gpu-backend cuda

# Force CPU (override auto-detection)
synthpass extract --image passport.png --gpu-backend cpu

# Limit GPU memory usage
synthpass extract --image passport.png --gpu-backend cuda --gpu-memory 2048
```

**Environment Variables:**
```bash
# For synthpass-serve
SYNTHPASS_OCR_BACKEND=cuda
SYNTHPASS_LLM_BACKEND=cuda
SYNTHPASS_GPU_MEMORY_MB=2048
```

### Doctor Command Enhancement
```bash
$ synthpass doctor
SynthPass v1.3.0
✅ MRZ engine: mrz v0.6.2
✅ OCR engine: ocrs v0.12.2, rten v0.24.0
  ├─ CPU backend: Available
  ├─ CUDA backend: Available (GTX 970, 3.5GB VRAM)
  └─ Vulkan backend: Available
✅ LLM engine: llama-cpp-sys v0.1.151
  ├─ CPU backend: Available
  └─ CUDA backend: Available
✅ License: Valid
```

---

## Determinism Guarantees

### Core Principle
**Same input → Same output, regardless of backend**

### Implementation Strategy

1. **Seeded RNG**: All random operations (if any) use seeded RNG
2. **Backend-independent algorithms**: Core logic unchanged
3. **Floating-point consistency**: GPU uses same numerical precision as CPU
4. **Fallback mechanism**: If GPU produces different results, fall back to CPU

### Validation
- Existing test suite must pass with both CPU and GPU
- Add cross-backend consistency tests
- Benchmark results include hash of outputs for verification

---

## Error Handling

### GPU Initialization Failures
```rust
/// Error types for GPU backend
#[derive(Debug, thiserror::Error)]
pub enum GpuError {
    #[error("GPU feature not compiled: {0}")]
    FeatureNotEnabled(&'static str),
    
    #[error("No compatible GPU found")]
    NoCompatibleGpu,
    
    #[error("Insufficient VRAM: need {0}MB, have {1}MB")]
    InsufficientVram(usize, usize),
    
    #[error("Driver error: {0}")]
    DriverError(String),
    
    #[error("CUDA error: {0}")]
    CudaError(#[from] rten::CudaError),
    
    #[error("Vulkan error: {0}")]
    VulkanError(String),
}
```

### Fallback Behavior
```rust
impl OcrBackend {
    pub fn load() -> Result<Self, GpuError> {
        match Self::auto() {
            Self::Cpu => Ok(Self::Cpu),
            Self::Cuda => Self::try_cuda().unwrap_or(Self::Cpu),
            Self::Vulkan => Self::try_vulkan().unwrap_or(Self::Cpu),
        }
    }
}
```

---

## Security Considerations

### Memory Safety
- GPU memory managed by Rust wrappers (rten, llama-cpp-sys)
- No unsafe code in SynthPass code
- FFI boundaries are well-defined and audited

### Data Privacy
- GPU processing is local (no cloud)
- Data never leaves device memory
- VRAM is cleared after use
- No telemetry or usage tracking

### Driver Security
- System-level drivers are user's responsibility
- SynthPass validates GPU capabilities before use
- Sandboxing via containerization recommended for production

---

## Rollout Strategy

### Step 1: Alpha (Development)
- Feature behind `--unstable-features` flag
- Requires explicit opt-in
- Documentation marked as experimental

### Step 2: Beta (Testing)
- Remove unstable flag
- Add to CLI help
- CI tests with GPU (optional job)

### Step 3: GA (General Availability)
- Full documentation
- Production-ready
- Recommended for GPU-equipped servers

---

## Monitoring & Metrics

### New Metrics
- `gpu_backend_used` (cpu/cuda/vulkan/metal)
- `gpu_memory_used_mb`
- `gpu_memory_total_mb`
- `inference_time_gpu_ms`
- `inference_time_cpu_ms`

### Health Check
```bash
GET /health
{
  "status": "healthy",
  "gpu": {
    "ocr_backend": "cuda",
    "llm_backend": "cuda",
    "vram_used_mb": 550,
    "vram_total_mb": 3584
  }
}
```

---

## Alternatives Considered

### Alternative 1: GPU-Only Build
**Rejected** - Breaks CPU-only deployments and offline-first principle

### Alternative 2: Runtime GPU Detection Without Features
**Rejected** - Would require dynamic loading, violates explicit dependency principle

### Alternative 3: Separate GPU Crates
**Rejected** - Adds complexity, harder to maintain consistency

### Alternative 4: Default to GPU
**Rejected** - Breaks existing deployments, not all hardware supports GPU

---

## Open Questions

1. **Should we support GPU for synthpass-gen?** (document generation)
   - Currently CPU-only, but could benefit from GPU for batch generation
   - Lower priority as generation is not performance-critical

2. **Should we add ROCm support for AMD GPUs?**
   - Yes, but as separate feature flag
   - Add in future iteration

3. **Should GPU be enabled by default on capable systems?**
   - No - explicit opt-in maintains control
   - Can revisit after extensive testing

4. **Should we add GPU-specific benchmarks to CI?**
   - Yes, but as optional jobs (require GPU hardware)
   - Use GitHub Actions runners with GPU

---

## Success Criteria

- [ ] GPU features compile without errors
- [ ] CPU path unchanged and all tests pass
- [ ] GPU path produces identical results to CPU (determinism)
- [ ] GPU path is faster than CPU (2x+ speedup)
- [ ] Fallback to CPU works when GPU unavailable
- [ ] CLI and API configuration works
- [ ] Documentation is complete
- [ ] CI passes with GPU features

---

## References

1. [rten CUDA documentation](https://docs.rs/rten/latest/rten/)
2. [llama-cpp-sys GPU features](https://docs.rs/llama-cpp-sys/latest/llama-cpp-sys/)
3. [CUDA Toolkit Documentation](https://docs.nvidia.com/cuda/)
4. [Vulkan API](https://www.vulkan.org/)
5. [SYNTHPASS_ENGINEERING_CONSTITUTION.md](SYNTHPASS_ENGINEERING_CONSTITUTION.md)
6. [knowledge/ROADMAP.md](ROADMAP.md)

---

## Revision History

| Date | Author | Changes |
|------|--------|---------|
| 2026-08-03 | devstral | Initial draft |
