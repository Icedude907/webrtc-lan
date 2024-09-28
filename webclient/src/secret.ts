import * as THREE from "three"

var screenwidth = window.innerWidth
var screenheight = window.innerHeight

// Create a renderer
var renderer = new THREE.WebGLRenderer({
  antialias: true,
});
renderer.setClearColor("#000000");
renderer.setSize( screenwidth, screenheight );
document.body.appendChild( renderer.domElement );

// Create a basic perspective camera
var camera = new THREE.PerspectiveCamera( 75, screenwidth/screenheight, 0.1, 1000 );
camera.position.z = 4;

window.addEventListener("resize", () => {
  screenwidth = window.innerWidth
  screenwidth = window.innerHeight
  //update camera
  camera.aspect = screenwidth / screenwidth
  camera.updateProjectionMatrix()
  renderer.setSize(screenwidth, screenwidth)
})

// Create an empty scene
var scene = new THREE.Scene();

var geometry = new THREE.TorusGeometry( 1 );
var material = new THREE.MeshBasicMaterial( { color: "#0056A1" } );
var spinner = new THREE.Mesh( geometry, material );

scene.add( spinner );

var render = ()=>{
  requestAnimationFrame( render );

  spinner.rotation.x += 0.01;
  spinner.rotation.y -= 0.01;

  renderer.render(scene, camera);
};

// Start
render();