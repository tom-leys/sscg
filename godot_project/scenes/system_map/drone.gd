extends KinematicBody

var test_mode = false

export var drone_active = false

var camera = null

var view_sensitivity = 0.3
var yaw = 0.0
var pitch = 0.0
var speed = 4.0
var grav = 3.0
var accel = 0.0
var jump_strength = 0.5
var jump_motion = Vector3(0, 0, 0)
var anti_grav = test_mode

var mining_vox = null
var mining_pos = null
var marker_vox = null
var mining_time = 0.0 # in seconds
var mining_time_dest = 1.0 # in seconds
var prev_vox_pos = null


var old_on_floor = false

func set_active(is_active, glob_point):
	drone_active = is_active
	if drone_active:
		self.show()
		self.get_parent().get_node("GUI/ShipControlsInfo").hide()
		self.get_parent().get_node("GUI/DroneControlsInfo").show()
		self.get_parent().get_node("GUI/CargoMeter").show()
		self.get_parent().get_node("GUI/DroneHUDInfo").show()
		camera.current = true;
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
		pitch = 0
		yaw = 180
		self.set_rotation(Vector3(deg2rad(pitch),deg2rad(yaw), 0))
		self.set_translation(glob_point)
	else:
		self.stop_mining()
		self.get_parent().get_node("GUI/ShipControlsInfo").show()
		self.get_parent().get_node("GUI/DroneControlsInfo").hide()
		self.get_parent().get_node("GUI/CargoMeter").hide()
		self.get_parent().get_node("GUI/DroneHUDInfo").hide()
		self.hide()

func _input(event):
	if !drone_active:
		return
		
	if Input.is_action_just_pressed("fly_forward"):
		#print("JUMP", self.get_transform().origin, jump_motion)
		jump_motion += Vector3(0, grav, 0) * jump_strength;
		
	if event is InputEventMouseMotion:
		yaw = fmod(yaw - event.relative.x * view_sensitivity, 360) 
		pitch = max(min(pitch - event.relative.y * view_sensitivity, 90),-90)
		self.set_rotation(Vector3(deg2rad(pitch),deg2rad(yaw), 0))

func process_movement(delta):
	var forw = -self.get_transform().basis.z;
	forw.y = 0;
	var righ = -self.get_transform().basis.x;
	var motion = Vector3()
	
	if Input.is_action_pressed("walk_forward"):
		motion += forw.normalized()
	if Input.is_action_pressed("fly_stop") or Input.is_action_pressed("walk_backward"):
		motion -= forw.normalized()
	if Input.is_action_pressed("turn_right"):
		motion += righ.normalized()
	if Input.is_action_pressed("turn_left"):
		motion -= righ.normalized()
	if Input.is_action_just_pressed("antigrav"):
		anti_grav = not anti_grav
		jump_motion = Vector3(0, 0, 0)
		
	var speed_factor = 1
	if Input.is_action_pressed("faster"):
		speed_factor = 3
		
	motion *= speed_factor
	
	if not anti_grav:
		jump_motion += Vector3(0, -grav, 0) * delta

	if jump_motion.dot(Vector3(0, 1, 0)) < -10.0:
		jump_motion = Vector3(0, -10.0, 0)
			
	motion += jump_motion;
	self.move_and_slide(motion * speed, Vector3(0, 1, 0))
	
	if self.is_on_floor() or self.is_on_ceiling():
		jump_motion = Vector3(0, 0, 0);
		
	old_on_floor = self.is_on_floor()

func stop_mining():
	if mining_vox:
		var raym = self.find_node("RayMesh")
		raym.hide()
		mining_vox.mine_status(false)
		mining_vox   = null
		mining_pos   = null
		prev_vox_pos = null

		self.get_parent().get_node("GUI/DroneHUDInfo/MiningProgress").hide()
		$MiningBeamSound.disable_beam()
		
func process_mining_gun(delta):
	var cast = self.get_child(0)
	var c = cast.get_collider()
	if c:
		var raym = self.find_node("RayMesh")
		var vox = c.get_parent()
		var p = cast.get_collision_point()
		var cn = cast.get_collision_normal()
		#d# var n = self.get_child(1)
		#d# n.global_transform.origin = p - (0.13 * cn)
		var vox_coord = vox.to_local(p - (0.1 * cn))
		var vv = Vector3(floor(vox_coord.x), floor(vox_coord.y), floor(vox_coord.z))
		
		var dir_vector = p - cast.global_transform.origin
		
		# raycast origin
		#  \        a 
		#   O----------------(p)
		# b |          -----
		#   |alpha ----
		#   o------
		#    \
		#     raymesh origin
		var a     = dir_vector.length() # distance of hit point from raycast
		var b     = 0.05                # distance of raymesh on Y axis
		var alpha = atan(a / b)         # correction angle for direction cylinder
		raym.set_rotation(Vector3(-alpha, deg2rad(90) - alpha, 0))
		
		var raymesh_vector = p - raym.global_transform.origin
		raym.scale.y = raymesh_vector.length() * 2
		
		if mining_pos and vv != mining_pos:
			stop_mining()
			
		vox.looking_at(vv.x, vv.y, vv.z)

		if prev_vox_pos != vv:
			var mining_info = vox.mine_info_at_cursor()
			self.get_parent().wl_cb("on_update_mining_hud", [mining_info])
			prev_vox_pos = vv

		if Input.is_action_pressed("mine"):
			if mining_vox != vox:
				if vox.mine_status(true):
					mining_vox = vox
#					var mining_info = mining_vox.mine_info_at_cursor()
					mining_pos = vv
					raym.show()
					mining_vox.set_marker_status(true, true)
					mining_time = 0.0
					self.get_parent().get_node("GUI/DroneHUDInfo/MiningProgress").show()
					self.get_parent().get_node("GUI/DroneHUDInfo/MiningProgress").value = 0.0
					marker_vox = vox
					$MiningBeamSound.enable_beam()
				else:
					vox.looking_at_nothing()
					mining_pos = null
			else:
				mining_time = mining_time + delta
				var done_value = (mining_time * 100.0) / mining_time_dest
				self.get_parent().get_node("GUI/DroneHUDInfo/MiningProgress").value = done_value
				if mining_vox and mining_time > mining_time_dest:
					mining_vox.mine_at_cursor()
					$MiningBeamSound.play_pop()
					stop_mining()
					#mining_vox.looking_at_nothing()
					#mining_time = 0.0
					#mining_pos = null
		else:
			vox.set_marker_status(true, false)
			marker_vox = vox
			stop_mining()
	else:
		if prev_vox_pos:
			self.get_parent().wl_cb("on_update_mining_hud", [null])
			prev_vox_pos = null
	
		if marker_vox:
			marker_vox.looking_at_nothing()
		stop_mining()

func _physics_process(delta):
	if !drone_active:
		return
		
	process_movement(delta)
	process_mining_gun(delta)

# Called when the node enters the scene tree for the first time.
func _ready():
	camera     = self.get_node("Camera")

# Called every frame. 'delta' is the elapsed time since the previous frame.
#func _process(delta):
#	pass
