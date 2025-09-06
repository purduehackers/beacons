var idNum = 1;


//create a few beacons
addBeacon({
	id: idNum++,
	title: `project 1`,
	desc: `we are working on project 1 reall hard over here`,
	baseColor: `#a260be`
});
addBeacon({
	id: idNum++,
	title: `project 2`,
	desc: `we are working on project 2 reall hard over here`,
	baseColor: `#1b713b`
});
//add a few unpaired beacons. They should auto-move to the end of the list
addBeacon({id: idNum++});
addBeacon({
	id: idNum++,
	title: `hands and feet and hands and feet and arms and legs and shouldbers`,
	desc: `what's up everyone it's your buddy victor frankenstein here back again with another human corpse project. The last one I made was such a smashing success I figured I needed to create a nother one and this time not give it childhood trauma`,
	baseColor: `#ff5675`
});
addBeacon({id: idNum++});
addBeacon({id: idNum++});
addBeacon({
	id: idNum++,
	title: `mine craft`,
	desc: `mine craft`,
	baseColor: `#10f71f`
});

//destroy a beacon
removeBeacon(2);

addBeacon({
	id: idNum++,
	title: `sewing pants by hand`,
	desc: `literal luddite mentality but hey! The luddites had a point. They were just also misguided unfortunately. So kind of negates their point there.`,
	baseColor: `#2f52be`
});
addBeacon({
	id: idNum++,
	title: `beacons`,
	desc: `sending out a beacon for working on beacons`,
	baseColor: `#42223f`
});


//send into pairing mode






//add some history items for display
addHistory({
	id: idNum++,
	user: 1,
	title: `The best project ever`,
	desc: `This is the best project. It will change the world a thousand times over. Millions will come from afar to watch the magnificence`,
	color: `#39ed3f`,
	date: 1,
});
addHistory({
	id: idNum++,
	user: 1,
	title: `The worst project ever`,
	desc: `After the success of our last venture, we tried to replicate the same thing this week. We failed miserably. This is that project. Witness, condemned to history, our failure.`,
	color: `#FF0000`,
	date: 2,
});
addHistory({
	id: idNum++,
	user: 1,
	title: `just a mediuim project`,
	desc: `yeag`,
	color: `#eab136`,
	date: 3,
});


addHistory({
	id: idNum++,
	user: 2,
	title: `The best project ever`,
	desc: `This is the best project. It will change the world a thousand times over. Millions will come from afar to watch the magnificence`,
	color: `#00FFFF`,
	date: 1,
});

addHistory({
	id: idNum++,
	user: 2,
	title: `Hands and feet`,
	desc: `have you ever wanted. them.`,
	color: `#0000FF`,
	date: 2,
});

