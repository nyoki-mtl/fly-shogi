# Local verification

Verified on Windows with Rust, Python 3.13 and the locked dependencies.
These checks cover software operation.

| Check | Result |
| --- | --- |
| Independent source directory | Built and ran without importing another project checkout |
| Official DL Suisho download | Extracted ONNX matches the SHA-256 in `teacher.md` |
| MaleCNS source validation | All three pinned input hashes passed |
| Full graph reconstruction | 166,700 neurons, 25,582,938 edges; exact pinned CSR hash |
| Neutral L-BFGS-B circuit | 77,364 plastic edges, 8,374 sources; all fixed fields match the actual 30% model |
| Neutral dopamine circuit and targeting | 61,210 plastic edges; 337 DAN target maps; all 97 MBONs covered |
| Rust checks | 12 tests passed, including objective/runtime agreement and analytic gradient checks; formatting passed |
| Python label/reward checks | 4 tests passed; 84 generated/hand-written positions, both turns, promotions and drops; non-public teacher rejection |
| UI build | TypeScript check, board build and 2 animation timing tests passed |
| Teacher preparation | Four synthetic fixtures labeled by the downloaded DL Suisho |
| L-BFGS-B training | Cache, 3 finite-difference checks, optimization and full-circuit validation completed; final model changed 7,344 gains |
| Dopamine training | Two training fixtures processed; 4,671 gains changed from neutral |
| Resume | Continued from 2 to 4 cumulative samples in a separate run |
| Evaluation | Loaded the trained model and evaluated both validation fixtures |
| Demo API | 6 legal plies with the actual 30% model; earlier dopamine smoke/pretrained checks also passed; illegal/wrong-side actions rejected |
| Reward consistency | Training records and demo policy probabilities matched exactly on all four fixtures |
| Model immutability | Default L-BFGS-B and alternative dopamine model hashes unchanged after play |
| Browser | Piece click, fly reply, reward, New game and both hand panels checked |

The four fixtures contain only two training positions. The L-BFGS-B final
iterate fits these training targets but worsens validation loss, so `best.json`
correctly retains the initial model. The full-circuit check is separate from
cached validation. The historical model is evaluated in its model card.
