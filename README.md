# Luminance render for imgui-rs

Use this renderer if you want to add imgui-rs to a luminance-based application.

See [luminance](https://github.com/phaazon/luminance-rs) and [imgui-rs](https://github.com/Gekkio/imgui-rs) for more details.

## How to use

Initialize the renderer before your main loop with:
```rust
let mut renderer = imgui_luminance::Renderer::new(&mut surface, &mut imgui);
```
where surface is the Gltw surface and imgui is the imgui Context.

At each frame, prepare the data to draw with:
```rust
renderer.prepare(&mut surface, draw_data);
```

Then render it to the framebuffer of your choice. For example, the backbuffer:
```rust
surface.pipeline_builder().pipeline(&back_buffer, &PipelineState::default(), |pipeline, mut shd_gate| {
    renderer.render( &pipeline, &mut shd_gate, draw_data);
});
```

The full example can be found [here](./examples/demo.rs).

## What's left?

The renderer might not be bug-free. I am using it on a different project so I'll fix any issues I find.
Don't hesitate to add tickets if something is wrong.

I will also update to the newest version of luminance when it is released so it should become
backend agnostic at that moment.