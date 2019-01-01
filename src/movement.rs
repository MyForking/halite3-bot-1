use super::GameState;
use hlt::direction::Direction;
use hlt::log::Log;
use hlt::position::Position;
use hlt::ShipId;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

pub fn greedy(state: &mut GameState, ship_id: ShipId) -> Direction {
    const PREFER_STAY_FACTOR: usize = 2;
    const HARVEST_LIMIT: usize = 10;
    const SEEK_LIMIT: usize = 50;

    let (pos, cargo) = {
        let ship = state.get_ship(ship_id);
        (ship.position, ship.halite)
    };

    let movement_cost =
        state.game.map.at_position(&pos).halite / state.game.constants.move_cost_ratio;

    if cargo < movement_cost {
        return Direction::Still;
    }

    let syp = state.me().shipyard.position;

    let current_halite =
        state.game.map.at_position(&pos).halite;

    let current_value =
        current_halite / state.game.constants.extract_ratio;

    let mut mov = Direction::get_all_cardinals()
        .into_iter()
        .map(|d| (d, pos.directional_offset(d)))
        .map(|(d, p)| {
            (
                state.game.map.at_position(&p).halite,
                state.game.map.at_position(&p).halite / state.game.constants.extract_ratio,
                d,
                p,
            )
        })
        .filter(|&(halite, _, _, _)| halite >= HARVEST_LIMIT)
        .filter(|&(_, _, _, p)| p != syp)
        .filter(|&(_, value, _, _)| value > movement_cost + current_value * PREFER_STAY_FACTOR)
        .filter(|(_, _, _, p)| state.navi.is_safe(p))
        .max_by_key(|&(_, value, _, _)| value)
        .map(|(_, _, d, p)| (d, p));

    // if there is nothing to gather, find new resource location
    if mov.is_none() && current_halite < SEEK_LIMIT {
        mov = state.get_nearest_halite_move(pos, SEEK_LIMIT).map(|d| (d, pos.directional_offset(d)));
        if let Some((_, p)) = mov {
            Log::log(&format!("greedy ship {:?} found new target: {:?}.", ship_id, p));
        } else {
            Log::log(&format!("greedy ship {:?} does not know where to go.", ship_id));
        }
    }

    let (d, p) = mov.unwrap_or((Direction::Still, pos));

    state.navi.mark_unsafe(&p, ship_id);
    d
}

pub fn thorough(state: &mut GameState, ship_id: ShipId) -> Direction {
    const HARVEST_LIMIT: usize = 50;

    let pos = state.get_ship(ship_id).position;

    let current_halite =
        state.game.map.at_position(&pos).halite;

    if current_halite >= HARVEST_LIMIT {
        return Direction::Still;
    }

    let mov = state.get_nearest_halite_move(pos, HARVEST_LIMIT).map(|d| (d, pos.directional_offset(d)));
    if let Some((_, p)) = mov {
        Log::log(&format!("thorough ship {:?} found new target: {:?}.", ship_id, p));
    } else {
        Log::log(&format!("thorough ship {:?} does not know where to go.", ship_id));
    }

    let (d, p) = mov.unwrap_or((Direction::Still, pos));

    state.navi.mark_unsafe(&p, ship_id);
    d
}

pub fn cleaner(state: &mut GameState, ship_id: ShipId) -> Direction {
    let pos = state.get_ship(ship_id).position;

    if state.game.map.at_position(&pos).halite > 0 {
        return Direction::Still;
    }

    match state.get_nearest_halite_move(pos, 1) {
        None => Direction::Still,
        Some(d) => {
            let p = pos.directional_offset(d);
            state.navi.mark_unsafe(&p, ship_id);
            d
        }
    }
}

pub fn return_naive(state: &mut GameState, ship_id: ShipId) -> Direction {
    let ship = state.get_ship(ship_id).clone();
    let dest = state.game.players[state.game.my_id.0].shipyard.position;
    state.navi.naive_navigate(&ship, &dest)
}

pub fn return_dijkstra(state: &mut GameState, ship_id: ShipId) -> Direction {
    let pos = state.get_ship(ship_id).position;

    let dest = state.me().shipyard.position;

    let path = state.get_dijkstra_path(pos, dest);

    let d = path.first().cloned().unwrap_or(Direction::Still);

    let p = pos.directional_offset(d);
    if !state.navi.is_safe(&p) {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}

pub fn kamikaze(state: &mut GameState, ship_id: ShipId) -> Direction {
    let pos = state.get_ship(ship_id).position;

    let dest = state.me().shipyard.position;

    let path = state.get_dijkstra_path(pos, dest);
    let d = path.first().cloned().unwrap_or(Direction::Still);
    let p = pos.directional_offset(d);

    if p == dest
        && state
            .game
            .ships
            .values()
            .filter(|ship| ship.owner != state.me().id)
            .any(|ship| ship.position == dest)
    {
        return d;
    }

    if !state.navi.is_safe(&p) {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}

pub fn go_home(state: &mut GameState, ship_id: ShipId) -> Direction {
    let pos = state.get_ship(ship_id).position;

    let dest = state.me().shipyard.position;

    let path = state.get_dijkstra_path(pos, dest);
    let d = path.first().cloned().unwrap_or(Direction::Still);
    let p = pos.directional_offset(d);

    if p == dest {
        return d;
    }

    if !state.navi.is_safe(&p) {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}

pub fn nn_collect(state: &mut GameState, ship_id: ShipId) -> Direction {
    let pos = state.get_ship(ship_id).position;

    const R: usize = 2;
    const N: usize = R * 2 + 1;

    let r = R as i32;

    let mut x = [0.0; N*N];
    let mut k = 0;
    for i in pos.x-r ..= pos.x+r {
        for j in pos.y-r ..= pos.y+r {
            x[k] = state.game.map.at_position(&Position{x:i, y:j}).halite as f64 / 1000.0;
            k += 1;
        }
    }

    let d = match state.collector_net.choice(&x[..]) {
        0 => Direction::Still,
        1 => Direction::East,
        2 => Direction::South,
        3 => Direction::West,
        4 => Direction::North,
        o => panic!("Invalid NN output: {}", o),
    };

    let p = pos.directional_offset(d);
    if !state.navi.is_safe(&p) {
        return Direction::Still;
    } else {
        state.navi.mark_unsafe(&p, ship_id);
        return d;
    }
}

#[derive(Debug)]
pub struct CollectorNeuralNet {
    layer1_weights: Vec<f64>,
    layer2_weights: Vec<f64>,
    hidden_activations: Vec<f64>,
    output_activations: Vec<f64>,
    n_input: usize,
    n_hidden: usize,
    n_output: usize,
}

impl CollectorNeuralNet {
    fn forward_step(&mut self, x: &[f64]) -> &[f64] {
        let mut w1it = self.layer1_weights.iter();
        for h in 0..self.n_hidden {
            let mut act = 0.0;
            for i in 0..self.n_input {
                act += x[i] * w1it.next().unwrap();
            }
            self.hidden_activations[h] = act.max(0.0);  // relu layer
        }

        let mut w2it = self.layer2_weights.iter();
        for o in 0..self.n_output {
            let mut out = 0.0;
            for h in 0..self.n_hidden {
                out += self.hidden_activations[h] * w2it.next().unwrap();
            }
            self.output_activations[o] = out;
        }

        &self.output_activations
    }

    fn choice(&mut self, x: &[f64]) -> usize {
        let out = self.forward_step(x);
        out.iter().enumerate().fold((99, 0.0), |acc, (i, &o)| {
            if o > acc.1 {
                (i, o)
            } else {
                acc
            }
        }).0
    }

    pub fn new() -> Self {
        CollectorNeuralNet {
            n_input: 25,
            n_hidden: 15,
            n_output: 5,
            layer1_weights: vec![2.0488975072708002, 0.7932895114585551, -0.7494349532130407, 0.7001908717219815, 1.3646419930097933, 1.271422888063959, 0.6724609220062753, -1.162655075670488, -0.02227274107289263, 0.754267497803526, 1.886214675739057, 2.6058608405006964, -12.720920409444924, -0.4981485984564489, 0.30649564188181816, 1.7238874958922565, 0.13696659566635838, -0.09642674531528818, -0.1172470715219906, 0.2708256113690572, 2.209088240575293, -0.4418598369883222, -0.6077762941457894, -0.40279651202277383, 0.2280239479320584, 3.4797768540942857, 0.5361325701044771, 0.3549094544869688, 0.17915628692684166, 1.4211797722922663, 1.855569059908291, 0.6478345999774199, 0.20757065672450317, -0.286949923603124, 0.8628721002297923, 2.429644247722757, 3.5981151462092638, -11.124495148466638, 0.5080714543280918, 0.7621057350196152, 1.378327177881915, -0.39809442397389316, -3.760045475670287, -0.02320544695749292, 0.8519431260724981, 1.1767883290113486, -0.5635040614484631, -1.6669310282989862, -0.32466285137398637, 0.49475350019076497, -0.19978128790616811, 0.018098899484667925, -0.18056271245211516, 0.04917608608793759, 0.03745633448405858, 0.12247604713830554, -0.08804248036205618, -0.048857370620712715, -0.17239977110345875, -0.5193864382068352, 0.136208019047245, -0.307380181072233, -0.27708720056223773, -0.5531301718091604, -0.2811514296032212, -0.12783822395565034, -0.23171611133091705, 0.2542325888085626, -0.11153911865789726, -0.17984403473346797, -0.08397886425501966, -0.09452557089582114, -0.11693653928541091, -0.16291834052114532, -0.10413404446809384, 1.7226735607165313, 1.105681177364736, 1.8254747408930772, 0.9189745710589254, 0.8098416391875143, 1.0718424915911369, 0.31876399660477056, 1.8502371271618137, -0.32650833535222085, 0.11454312437859399, 0.11614748395670967, -2.1286821545848222, -12.723250794678483, 0.017305903219807687, -0.11896619332725127, 0.6397426107554917, 0.7746248474861119, 1.1468290625768367, -0.12343458311167185, -0.6593129103392714, 1.9395432703367592, 1.6491077357707868, 2.6195548853006967, 0.3367379310577306, -0.1088942023968197, 0.7029833660469549, 0.616065963285355, 0.851294037736299, -0.8608994112291676, -0.40296027221521913, 1.3760282625887243, 0.7028665345995483, 1.8196955268791253, 0.05490402930191439, -0.26795576489057166, 1.865452054651559, 2.3820181886398957, 15.404325237555142, 0.004009631403055214, 0.2474596414653115, -0.254141344841773, -0.5362210939861389, -3.066909417803359, -0.24126189062573394, 0.2149984183510073, -0.9522044514978938, -0.5893082343296778, -2.086352943299787, -0.7103193184268047, -0.609509686630584, 3.5148609761010716, 2.617351573655686, 5.563519196949713, 1.1394271953574822, 1.9946130774467943, 0.9103804420877104, 2.1694886243196696, 8.072087326279245, 0.03605448521461179, -0.14553955035132732, -1.2181562541289295, -3.774103148760719, -7.139767078694656, -0.20854556038391361, 0.7247518793994593, -2.968182165917094, -1.0427583816941854, -2.3786629825345136, -0.35379397343816554, 0.45819506012541156, -2.0962565315687467, -0.4040368933571812, -0.18486608957393524, 0.4787691526142992, 0.6176396663610215, -2.601513791484311, -0.6988694518991766, -0.1461077909002154, -0.45310106563721136, -1.2020976217977652, -1.106518638537277, 0.8167905183415438, 0.8367350122068519, 0.5845939849210475, -0.11416737511801664, -1.897701380330609, -3.495256034618078, 10.730141329450989, 0.6478751990125138, 0.32869598628771957, -0.7850329158107561, 0.8198452480758133, 4.851685818672719, -0.053731999801076076, 0.16657913234520683, -0.9494370688662864, 1.289112334445368, 2.9757679798178978, 1.1041072765785838, 0.6140212637531313, -1.9556838102488772, -0.7972564221876624, 0.6681243199184316, -0.2594238073441878, -1.3261633653790035, -0.9557280526037552, 0.44277715342060076, 2.243745260656941, 0.9355073057836978, -0.05452643319953212, -1.2672581867544008, -2.828254131322428, 13.231760884740742, 1.0373275219573732, 0.14716968884801743, -1.6467419544958721, 1.1386954849725501, 4.001935706733098, 0.9875888447723289, -0.07589348760749239, -2.036463123719775, 0.021736913880338233, 1.4017034961879342, 0.7203467649161311, -0.08021554713242149, 0.5053933688993788, 0.29639800124440435, 0.050676549376177445, -0.2893032301256404, -1.556673786607969, 1.3316669915814008, 2.551832692976115, 2.2855298559056245, 0.44589674305277266, 0.0030657206554082763, -0.05511959248191498, -3.1039070848868953, -12.85948013679609, -0.14793732094966128, -0.23593729551394954, 1.9521492690031172, 2.4619704402417253, 3.4649549568673885, -0.5735948718338524, -0.3538807301621037, 2.2694949611395656, 1.2730300901836775, 1.6081734386284456, 1.1441037266294407, 1.1349227587411006, -0.17319021004526072, 0.11180486014695867, -0.29081384477566885, -0.22921205025234345, -0.265223571542764, -0.013450674343363254, -0.1615526034042932, -0.34814298996251963, 0.23972727360343318, -0.1144599801461973, -0.4515942752046948, -0.4322396378250541, -0.12289812561577382, -0.3098727510510434, 0.20366277293257146, 0.19444277428563184, 0.00558609958768802, -0.10113256184538215, -0.010490715426341248, -0.22355932229470205, -0.3199249280426354, -0.37712911744198185, -0.19117966390206115, -0.32852778469224303, -0.03228682852595365, 2.578957006078649, 0.28827824381860856, -2.0988632202049002, -0.36905906608349964, -0.6987836097691326, 2.1572420096052904, 1.1117355956929833, -3.714207325306401, 0.024412625048803413, 0.14646519428581478, 3.4928545282270202, 6.118854658316352, -8.248862297971765, -0.32759037559130727, 0.19972662979516087, 2.3707446491148336, 0.23398243083021059, -2.9512813857644593, 1.0063350607031327, 1.0272430846358158, 2.705938142758787, -0.7876580304706424, -2.3080448903322743, -0.12700590373844378, 0.27739399280105786, 0.45860637981048963, 0.4636078952721182, -1.0148331837192959, -0.5475421529025084, 0.18203937994980565, 1.1696376395095205, 1.4069352333143887, -1.3042273246996101, 0.06406290294397894, -0.2209385554389975, 1.8622682804697885, 3.349691531817086, 14.704764982157958, -0.6280399908929135, -0.07182698980791204, 0.7130347487924712, -0.09025935424919819, -3.3598251378257746, -0.35462245725383995, 0.07829004718693135, -1.0040903735039133, -0.3093845552577307, -2.746641931252896, -1.5961228540985763, -1.4250122851963158, 0.26985224461121704, 0.34458852144673885, 0.7844670212989981, 0.6464277609977682, -0.09615337124183702, 1.1131311150245518, 1.6549242606773784, 1.2554859851915683, 1.1271173691233076, 0.44286409527849796, 1.0348733010230773, 1.3191122029330877, 16.01104890156455, 0.9406448379981862, 0.8316314033985867, 0.06217272731953863, 0.8572472078843275, 0.4619358981014523, 1.3900917599299536, 0.6392743286250945, -0.34056647300413134, 0.8751438756077791, 0.41799815362056364, 0.41804183545942886, -0.06286492136678137, -1.205187126173215, -3.058744029032776, -2.10043086739971, 1.2601679435048139, -0.08762224639497465, 0.15665910232493954, -2.9935420243741366, -5.503934356878268, 0.9957837590978512, 1.0681542493185736, 2.946460741043917, 0.4720896026329954, -4.4093090522581795, 0.7333030492216623, 1.1759438125723856, 3.0284625134762706, 1.270642681500833, 2.2066453926953504, 0.25491848515846616, 0.7914646964449464, 4.058935849696385, 1.471296258837811, 2.3262071738453414, 0.2070026370629401, -0.07444176979013362, 0.1442065515598784, -0.0409037243956416, -3.690905212526691, -1.990151853813337, -0.9459567761291134, 0.18218443399852388, -0.29973450430582893, -7.4919364072650145, 0.3546228563489111, 0.15532860572530824, 0.5792633055319258, 3.5824386410417883, 2.911163202298319, 0.6418813508174468, -0.21499729595919612, 2.5524159391082746, 0.519925566870954, 3.6684703591278507, -0.06372611736503664, -0.43970965386407207, 1.2558254226375989, -0.4425095217051703, 1.1516909057203446, -0.08299994299026922, 0.27765004079107947],
            layer2_weights: vec![-9.691367905640083, -6.009556473721567, -0.22499078650755963, -6.711995446621867, 7.493687060274628, -2.014735470973805, 7.12328518310814, 8.855478096927088, -9.670769211021263, 0.3154805175849566, -3.101239669038399, 8.053517218726363, 9.295992921218717, -0.900939227905212, 3.7538170936668473, 5.522910746681954, 0.21490741704369393, 0.23387254770423824, 4.79103172143686, -2.720164896509874, -0.5770973693314893, 5.9327922721722475, 3.152804931500022, 10.441076848577866, 0.1652996910202581, 2.6672271635492986, -3.3647394746894843, -0.13729508103143187, 10.123987407530006, 6.6834774317793695, 2.844388283423871, 0.5979499607918172, 0.24521219763893376, 1.004789482902983, -2.0698184566427726, 3.311339189283877, 0.9012815997266364, 2.955876658847291, 6.6075724261746, -0.23104324649449126, 2.3274086853652864, 3.7407399603623745, -1.6980654526324586, 6.916153173322643, 5.379219193838071, 5.865613386350487, 6.719341016743353, 0.19984756807034715, 6.216178812314847, 2.2006500001800626, 7.999663268340657, 0.9912451207733828, 2.9057156853645756, 10.877488391849536, -0.03450165101332448, 2.096261770879177, 0.9342327124606968, 0.7696041750037967, -19.742206165324735, -12.040727390322406, 8.453056812654618, 8.526620243377076, -0.30020131730180993, 3.0761048873652315, 2.3254111048657333, 0.8480285107646407, -2.5225807296253144, -1.6520654500800946, 3.6727220575816384, 0.03923430357729951, 8.052203873747418, 3.72896742801937, 0.5735201847859409, 8.420661381635389, 3.7120790628911253],
            hidden_activations: vec![0.0; 15],
            output_activations: vec![0.0; 5],
        }
    }

    pub fn from_file(filename: &str) -> Self {
        let mut n_input = 0;
        let mut n_hidden = 0;
        let mut n_output = 0;
        let mut layer1_weights = vec![];
        let mut layer2_weights = vec![];

        let file = File::open(filename).expect(&format!("Could not open file {}", filename));
        let mut buf_reader = BufReader::new(file);

        let mut line = String::new();
        while buf_reader.read_line(&mut line).unwrap() > 0 {
            let mut it = line.split(": ");

            let var = it.next().unwrap();
            let val = it.next().unwrap();

            match var {
                "n_input" => n_input = val.parse().unwrap(),
                "n_hidden" => n_hidden = val.parse().unwrap(),
                "n_output" => n_output = val.parse().unwrap(),
                "layer1_weights" => layer1_weights = val.split(", ").map(str::parse).map(Result::unwrap).collect(),
                "layer2_weights" => layer2_weights = val.split(", ").map(str::parse).map(Result::unwrap).collect(),
                _ => panic!("invalid name: {}", var),
            }
        }
        let mut hidden_activations = vec![0.0; n_hidden];
        let mut output_activations = vec![0.0; n_output];

        CollectorNeuralNet {
            n_input,
            n_hidden,
            n_output,
            layer1_weights,
            layer2_weights,
            hidden_activations,
            output_activations
        }
    }
}

