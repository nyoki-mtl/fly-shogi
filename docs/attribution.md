# Attribution

## MaleCNS v1.0

[MaleCNS](https://male-cns.janelia.org/) is a collaboration between FlyEM at
HHMI Janelia, the University of Cambridge Department of Zoology, the MRC
Laboratory of Molecular Biology and Google Research, with their collaborators.
The [official downloads](https://male-cns.janelia.org/download/) are licensed
under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

Our processing selects annotated non-glial neurons, orders body IDs, converts
contacts to CSR, assigns model signs from neurotransmitter predictions and
adds artificial stimulation, decoding and synaptic gains. Display coordinates
are normalized and sampled from annotated soma positions. These are changes
to the data representation and model.

## Implementation sources

| Component | Source | Terms and use |
| --- | --- | --- |
| Board, both hand panels, pieces | [Included ShogiLens source](../ui/vendor/shogilens/), revision `5b508c3b39fdad12ba8f6c0e1a74ade7a8fdb4f0` | MIT; copied/adapted sources and 28 original piece images. Notices in `web/vendor/shogilens/`. |
| Rules and move labels | [rsshogi 1.2.4](https://crates.io/crates/rsshogi/1.2.4) | MIT; pinned Cargo dependency. |
| Teacher feature encoder | [cshogi 1.0.4](https://github.com/TadaoYamaoka/cshogi) | GPL-3.0; Python dependency, used for dlshogi features and an independent label check. |
| UI runtime | React, React DOM | MIT; license texts retained in `web/vendor/shogilens/`. Sources are built with the pinned npm lockfile. |
| Numerical reference | [Shiu et al. (2024)](https://doi.org/10.1038/s41586-024-07763-9), [Drosophila brain model](https://github.com/philshiu/Drosophila_brain_model) | Background for spiking dynamics; the Rust implementation is local project code. |
| Data hash reference | [Doomfly](https://github.com/nftechie/doomfly), revision `71ecf53d78eaffaf1a57ed7b0ccf5d458abc9f33` | Hashes used as a cross-check; downloads are from official MaleCNS storage. No game content is included. |
| Reward teacher | [DL Suisho by tayayan](https://www.youtube.com/watch?v=Z-wpDN-mBHI) | Separate download from the author. |

All dependency versions are recorded in `Cargo.lock`, `uv.lock` and
`package-lock.json`. Dependency licenses retain their own terms. The source package contains code, documentation, UI assets and small test fixtures. Fly weights are distributed separately through GitHub
Releases, with a model card, checksums and explicit weight-distribution terms.
