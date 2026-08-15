pragma Ada_2012;
with Ada.Numerics.Elementary_Functions; use Ada.Numerics.Elementary_Functions;
with Ada.Numerics.Float_Random; use Ada.Numerics.Float_Random;

with raylib_h; use raylib_h;
with Interfaces.C; use Interfaces.C;
with Interfaces.C.Strings; use Interfaces.C.Strings;

procedure Asteroids is

   type Asteroid_Index is range 1..64;
   type Bullet_Index is range 1..32;

   SCREEN_WIDTH : constant := 800;
   SCREEN_HEIGHT : constant := 600;
   SHIP_SIZE : constant := 20.0;
   SHIP_ROTATION_SPEED : constant := 3.5;
   SHIP_THRUST : constant := 220.0;
   SHIP_DRAG : constant := 0.60;
   MAX_SHIP_SPEED : constant := 380.0;
   SHIP_COLLISION_RADIUS : constant := 12.0;
   BULLET_SPEED : constant := 520.0;
   BULLET_LIFETIME : constant := 1.1;
   FIRE_COOLDOWN : constant := 0.25;
   BULLET_RADIUS : constant := 2.0;
   ASTEROID_SPEED_MIN : constant := 40.0;
   ASTEROID_SPEED_MAX : constant := 110.0;
   ASTEROID_SPLIT_COUNT : constant := 2;
   STARTING_LIVES : constant := 3;
   STARTING_ASTEROIDS : constant  := 4;
   INVULNERABILITY_TIME : constant := 2.0;
   RESPAWN_DELAY : constant := 1.5;

   type Asteroid_Size is (Small, Medium, Large);

   Asteroid_Radius: constant array(Asteroid_Size) of Float := ( Small => 12.0, Medium => 22.0, Large => 40.0 );
   Asteroid_Score:  constant array(Asteroid_Size) of Integer := ( Small => 100, Medium => 50, Large => 20 );

   subtype Vec2 is Vector2;  -- Vector2 from raylib

   function "+"(A,B: Vec2) return Vec2 is ( A.X + B.X, A.Y + B.Y );
   function "-"(A,B: Vec2) return Vec2 is ( A.X - B.X, A.Y - B.Y );
   function "*"(A: Vec2; B: Float) return Vec2 is ( A.X * B, A.Y * B );
   function Length(A: Vec2) return Float is ( Sqrt(A.X * A.X + A.Y * A.Y) );

   type Ship_Record is record
      position:           Vec2    := ( Float(SCREEN_WIDTH / 2) , Float(SCREEN_HEIGHT / 2) );
      velocity:           Vec2    := (0.0, 0.0);
      rotation:           Float   := 0.0;
      alive:              Boolean := true;
      invulnerable_timer: Float   := INVULNERABILITY_TIME;
   end record;

   type Bullet is record
      position: Vec2    := (0.0, 0.0);
      velocity: Vec2    := (0.0, 0.0);
      lifetime: Float   := 0.0;
      active:   Boolean := false;
   end record;

   type Asteroid is record
      position:       Vec2    := (0.0, 0.0);
      velocity:       Vec2    := (0.0, 0.0);
      size:           Asteroid_Size := Small;
      rotation:       Float   := 0.0;
      rotation_speed: Float   := 0.0;
      active:         Boolean := false;
   end record;

   type Game_State is (Playing, Game_Over);

   type Bullet_Array is array(Bullet_Index) of Bullet;
   type Asteroid_Array is array(Asteroid_Index) of Asteroid;

   type Game is record
      ship:          Ship_Record;
      bullets:       Bullet_Array;
      asteroids:     Asteroid_Array;
      score:         Integer := 0;
      lives:         Integer := STARTING_LIVES;
      wave:          Integer := 1;
      fire_cooldown: Float   := 0.0;
      respawn_timer: Float   := 0.0;
      state:         Game_State := Playing;
   end record;

   -- Raylib: gcc's -fdump-ada-spec did not process the color #defines's from raylib.h
   BLACK  : constant Color := (0, 0, 0, 255);
   WHITE  : constant Color := (255, 255, 255, 255);
   GRAY   : constant Color := (130, 130, 130, 255);
   YELLOW : constant Color := (253, 249, 0, 255);
   RED    : constant Color := (230, 41, 55, 255);
   -- Raylib: wrap these functions because gcc's -fdump-ada-spec uses chars_ptr instead of char_array, which is not compatible with To_C()
   -- New_String() does return a chars_ptr, but needs to be Free()'ed as it is heap allocated.
   procedure DrawText (text : string; posX : int; posY : int; fontSize : int; the_color : Color) is
      text_arr  : aliased char_array := To_C(text);
      text_cstr : constant chars_ptr := To_Chars_Ptr(text_arr'Unchecked_Access);
   begin
      DrawText (text_cstr, posX, posY, fontSize, the_color);
   end;
   function MeasureText (text: string; fontSize: int) return int is
      text_arr  : aliased char_array := To_C(text);
      text_cstr : constant chars_ptr := To_Chars_Ptr(text_arr'Unchecked_Access);
   begin
      return MeasureText(text_cstr, fontSize);
   end;
   -- Raylib: cast KeyboardKey to int.
   function IsKeyDown (key : KeyboardKey) return Boolean is ( Boolean(IsKeyDown(int(key))) );

   -- Random number generators
   Rng: Ada.Numerics.Float_Random.Generator;

   procedure Spawn_Asteroid (g: in out Game; position: Vec2; size: Asteroid_Size) is
      angle : Float := Random(Rng) * 2.0 * PI;
      speed : Float := ASTEROID_SPEED_MIN + Random(Rng)*(ASTEROID_SPEED_MAX - ASTEROID_SPEED_MIN) ;
   begin
      for Asteroid of g.asteroids loop
         if not Asteroid.active then
            Asteroid := (
               position => position,
               velocity => ( Cos(angle) * speed, Sin(angle) * speed ),
               size => size,
               rotation => Random(Rng) * 2.0 * PI,
               rotation_speed => (Random(Rng) - 0.5) * 4.0,
               active => true
            );
            return;
         end if;
      end loop;
   end;

   function CircleCollide (A: Vec2; RA: Float; B: Vec2; RB: Float) return Boolean is ( Length(A-B) < RA + RB );

   function Wrap_Position (pos: in Vec2) return Vec2 is
      p: Vec2 := pos;
   begin
      if p.x < 0.0 then p.x := p.x + Float(SCREEN_WIDTH); end if;
      if p.x > Float(SCREEN_WIDTH) then p.x := p.x - Float(SCREEN_WIDTH); end if;
      if p.y < 0.0 then p.y := p.y + Float(SCREEN_HEIGHT); end if;
      if p.y > Float(SCREEN_HEIGHT) then p.y := p.y - Float(SCREEN_HEIGHT); end if;
   	return p;
   end;

   procedure Spawn_Wave (g: in out Game) is
      Count: Integer := STARTING_ASTEROIDS + (g.wave - 1);
      pos: Vec2;
   begin
      for I in 1 .. Count loop
         if Random(Rng) < 0.5 then
            pos.x := (if Random(Rng) < 0.5 then 0.0 else Float(SCREEN_WIDTH));
            pos.y := Random(Rng) * Float(SCREEN_HEIGHT);
         else
            pos.x := Random(Rng) * Float(SCREEN_WIDTH);
            pos.y := (if Random(Rng) < 0.5 then 0.0 else Float(SCREEN_HEIGHT));
         end if;
         Spawn_Asteroid (g, pos, Large);
      end loop;
   end;

   procedure Init_Game (g: in out Game) is
   begin
      g := ( others => <> );  -- (re)set to default values
      Spawn_Wave (g);
   end;

   procedure Fire_Bullet (g: in out Game) is
   begin
      if g.fire_cooldown > 0.0 then
         return;
      end if;

      for b of g.bullets loop
         if not b.active then
            declare
               facing : Vec2 := (Sin(g.ship.rotation), -Cos(g.ship.rotation) );
               nose : Vec2 := g.ship.position + facing * SHIP_SIZE;
            begin
               b := (
                  position => nose,
                  velocity => facing * BULLET_SPEED,
                  lifetime => BULLET_LIFETIME,
                  active   => true
               );
               g.fire_cooldown := FIRE_COOLDOWN;
               return;
            end;
         end if;
      end loop;
   end;

   procedure Split_Asteroid (g: in out Game; a: in out Asteroid) is
   begin
	   g.score := g.score + Asteroid_Score(a.size);
      if a.size /= Small then
         for i in 1 .. ASTEROID_SPLIT_COUNT loop
            Spawn_Asteroid (g, a.position, Asteroid_Size'Pred(a.size) );
         end loop;
      end if;
      a.active := false;
   end;

   procedure Update (g: in out Game; dt: Float) is
   begin
      if g.state = Game_Over then
         if IsKeyPressed(int(KeyboardKey_KEY_ENTER)) then
            Init_Game (g);
         end if;
         return;
      end if;

      if g.respawn_timer > 0.0 then
         g.respawn_timer := g.respawn_timer - dt;
         if g.respawn_timer <= 0.0 then
            g.ship.position := ( Float(SCREEN_WIDTH) / 2.0, Float(SCREEN_HEIGHT) / 2.0);
            g.ship.velocity := (0.0, 0.0);
            g.ship.rotation := 0.0;
            g.ship.alive := true;
            g.ship.invulnerable_timer := INVULNERABILITY_TIME;
         end if;
      else
         if IsKeyDown(KeyboardKey_KEY_A) or IsKeyDown(KeyboardKey_KEY_LEFT) then g.ship.rotation := g.ship.rotation - SHIP_ROTATION_SPEED * dt; end if;
         if IsKeyDown(KeyboardKey_KEY_D) or IsKeyDown(KeyboardKey_KEY_RIGHT) then g.ship.rotation := g.ship.rotation + SHIP_ROTATION_SPEED * dt; end if;

         if IsKeyDown(KeyboardKey_KEY_W) or IsKeyDown(KeyboardKey_KEY_UP) then
            declare
               facing : Vec2 := (Sin(g.ship.rotation), -Cos(g.ship.rotation));
            begin
               g.ship.velocity := g.ship.velocity + facing * (SHIP_THRUST * dt);
            end;
         end if;

         if IsKeyDown(KeyboardKey_KEY_SPACE) then fire_bullet(g); end if;

         declare
            speed : Float := Length(g.ship.velocity);
            drag_factor : Float := SHIP_DRAG ** dt;
         begin
            if speed > MAX_SHIP_SPEED then
               g.ship.velocity := g.ship.velocity * (MAX_SHIP_SPEED / speed);
            end if;

            g.ship.velocity := g.ship.velocity * drag_factor;

            g.ship.position := g.ship.position + g.ship.velocity * dt;
            g.ship.position := wrap_position(g.ship.position);
         end;

      end if;

      if g.fire_cooldown > 0.0 then
         g.fire_cooldown := g.fire_cooldown - dt;
      end if;

      for b of g.bullets loop
         if b.active then
            b.position := b.position + b.velocity * dt;
            b.lifetime := b.lifetime - dt;
            if b.lifetime <= 0.0
               or b.position.x < 0.0 or b.position.x > Float(SCREEN_WIDTH)
               or b.position.y < 0.0 or b.position.y > Float(SCREEN_HEIGHT)
            then
               b.active := false;
            end if;
         end if;
      end loop;

      for a of g.asteroids loop
         if a.active then
            a.position := a.position + a.velocity * dt;
            a.position := Wrap_Position(a.position);
            a.rotation := a.rotation + a.rotation_speed * dt;
         end if;
      end loop;

      for b of g.bullets loop
         if b.active then
            for a of g.asteroids loop
               if a.active and then CircleCollide(b.position, BULLET_RADIUS, a.position, Asteroid_Radius(a.size)) then
                  b.active := false;
                  Split_Asteroid (g, a);
                  exit;
               end if;
            end loop;
         end if;
      end loop;

      if g.ship.alive and g.ship.invulnerable_timer <= 0.0 then
         for a of g.asteroids loop
            if a.active and then CircleCollide(g.ship.position, SHIP_COLLISION_RADIUS, a.position, Asteroid_Radius(a.size)) then
               g.lives := g.lives - 1;
               g.ship.alive := false;
               if g.lives <= 0 then
                  g.state := Game_Over;
               else
                  g.respawn_timer := RESPAWN_DELAY;
               end if;
               exit;
            end if;
         end loop;
      end if;

      if g.ship.invulnerable_timer > 0.0 then g.ship.invulnerable_timer := g.ship.invulnerable_timer - dt; end if;

      declare
         any_asteroids : Boolean := (for some a of g.asteroids => a.active);
      begin
         if (not any_asteroids) and g.state = Playing then
            g.wave := g.wave + 1;
            Spawn_Wave (g);
         end if;
      end;
   end;

   procedure Draw_Ship (g: Game) is
   begin
      if not g.ship.alive or else (g.ship.invulnerable_timer > 0.0 and Integer(GetTime * 10.0) mod 2 = 0) then
         return;
      end if;

      declare
         nose : Vec2 := (g.ship.position.x + Sin (g.ship.rotation) * SHIP_SIZE, g.ship.position.y - Cos(g.ship.rotation) * SHIP_SIZE);
         left : Vec2 := (g.ship.position.x + Sin(g.ship.rotation + 2.5) * SHIP_SIZE, g.ship.position.y - Cos(g.ship.rotation + 2.5) * SHIP_SIZE );
         right: Vec2 := (g.ship.position.x + Sin(g.ship.rotation - 2.5) * SHIP_SIZE, g.ship.position.y - Cos(g.ship.rotation - 2.5) * SHIP_SIZE );
      begin
         DrawTriangleLines(nose, left, right, WHITE);
      end;
   end;

   procedure Draw (g: Game) is
   begin
      BeginDrawing;
      ClearBackground (BLACK);

      Draw_Ship (g);

      for a of g.asteroids loop
         if a.active then
            DrawCircleLines (int(a.position.x), int(a.position.y), Asteroid_Radius(a.size), GRAY);
         end if;
      end loop;

      for b of g.bullets loop
         if b.active then
            DrawCircleV (b.position, BULLET_RADIUS, YELLOW);
         end if;
      end loop;

      DrawText ("SCORE" & g.score'Img, 10, 10, 20, WHITE);
      DrawText ("LIVES" & g.lives'Img, 10, 34, 20, WHITE);
      DrawText ("WAVE" & g.wave'Img, 10, 58, 20, WHITE);

      if g.state = Game_Over then
         declare
            msg : constant string := "GAME OVER";
            sub : constant string := "Press ENTER to restart";
            w   : int := MeasureText(msg, 40);
            w2  : int := MeasureText(sub, 20);
         begin
            DrawText (msg, SCREEN_WIDTH / 2 - w / 2, SCREEN_HEIGHT / 2 - 40, 40, RED);
            DrawText (sub, SCREEN_WIDTH / 2 - w2 / 2, SCREEN_HEIGHT / 2 + 10, 20, WHITE);
         end;
      end if;

      EndDrawing;
   end;

   g: Game;

begin
   InitWindow (SCREEN_WIDTH, SCREEN_HEIGHT, New_String("Asteroids - Ada") );
   SetTargetFPS (60);

   Reset (Rng);
   Init_Game (g);

   while not WindowShouldClose loop
      Update (g, GetFrameTime);
      Draw (g);
   end loop;
   CloseWindow;
end;
