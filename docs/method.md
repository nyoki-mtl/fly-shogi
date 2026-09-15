# Model and learning

## Biological substrate

The MaleCNS v1.0 graph retains non-glial annotations with an assigned superclass,
sorted by body ID: 166,700 neurons and 25,582,938 directed edges representing
124,177,617 contacts. All retained connections participate in simulation.
The display samples available soma coordinates.

Contact signs follow neurotransmitter predictions: GABA, glutamate and
histamine are inhibitory; other or missing assignments are excitatory.
These signs and leaky integrate-and-fire dynamics are model assumptions.

## Artificial input and output

A turn-relative categorical encoding has 2,326 features. Fixed assignments
drive Kenyon cells with eight replicas per feature. Simulation runs for 40 ms
in 0.1 ms steps. Activity is collected in four time bins and decoded through a
fixed structured code into 1,496 move scores. Legal-move argmax selects the move.
Input assignment, timing and decoding are engineered interfaces.

Both methods use 4,064 Kenyon cells and 97 MBONs. Learned gains remain within
[0.05, 3.0] on existing connections; input and output codes remain fixed.

## L-BFGS-B

`prepare_circuit.py` initializes 77,364 excitatory MBON input connections from
8,374 source neurons at gain 1.0 (`mbon-input-v1`). `train_lbfgs.py` caches
full-circuit presynaptic activity, then minimizes legal-move cross-entropy
against the teacher's highest-probability move at fly temperature 0.02.
Analytic gradients differentiate the fixed-activity approximation.
Coordinate scaling aids optimization. Saved gains use float32.

The cache uses 33,496 bytes per position and is loaded into RAM. `best.json`
is selected by cached validation agreement, with loss breaking ties. The
initial model is eligible. The selected model is then evaluated with the full
circuit in `full-validation.json`. Independent evaluation uses separate games.
Another run with `--initial` recomputes activity under the saved gains.

## Dopamine-inspired local learning

`prepare_circuit.py --method dopamine` prepares the 61,210 KC-to-MBON edges
and DAN targeting map (`kc-mbon-v2`). `train.py` samples actions from the fly's
softmax at temperature 0.02 with 10% uniform exploration. Each action receives
its teacher policy probability as a scalar reward. The default schedule
averages 64 sampled-action updates per position using pre-update activity.

The fly's probability for the action serves as a predicted-approval baseline.
Reward minus baseline, fixed action codes and local eligibility traces control
nonnegative early/late pulses. Existing DAN-to-MBON contacts provide a normalized
targeting proxy. This is an abstract reward-routing model.

The update combines saturated eligibility, temporal credit, target normalization
and local competition. `config/dopamine.json` defines the schedule. Teacher
argmax and cross-entropy provide diagnostics for this local learning rule.

## Visualization

Neural glow displays simulated spike counts at sampled soma coordinates.
Nectar fills in proportion to DL Suisho's probability for the selected move;
a particle travels to the brain, followed by a gold wave. These are illustrations
of external reward. The saved gains remain fixed during play.

Teacher agreement measures matches with the reference moves. The model cards
provide the evaluation conditions for each checkpoint.
