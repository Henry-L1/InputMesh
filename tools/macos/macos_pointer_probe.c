#include <ApplicationServices/ApplicationServices.h>
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static long parse_long(const char *value, const char *name) {
  char *end = NULL;
  errno = 0;
  long parsed = strtol(value, &end, 10);
  if (errno != 0 || end == value || *end != '\0') {
    fprintf(stderr, "invalid %s: %s\n", name, value);
    exit(2);
  }
  return parsed;
}

static void print_position(void) {
  CGEventRef event = CGEventCreate(NULL);
  if (event == NULL) {
    fprintf(stderr, "could not read pointer position\n");
    exit(1);
  }
  CGPoint point = CGEventGetLocation(event);
  printf("%.0f %.0f\n", point.x, point.y);
  CFRelease(event);
}

static void print_displays(void) {
  CGDirectDisplayID displays[32];
  uint32_t count = 0;
  if (CGGetOnlineDisplayList(32, displays, &count) != kCGErrorSuccess) {
    fprintf(stderr, "could not enumerate displays\n");
    exit(1);
  }
  for (uint32_t index = 0; index < count; index++) {
    CGRect bounds = CGDisplayBounds(displays[index]);
    printf("%u %.0f %.0f %.0f %.0f\n", displays[index], bounds.origin.x,
           bounds.origin.y, bounds.size.width, bounds.size.height);
  }
}

static void move_pointer(int argc, char **argv) {
  if (argc != 8) {
    fprintf(stderr,
            "usage: macos_pointer_probe move x y delta_x delta_y count interval_ms\n");
    exit(2);
  }
  long x = parse_long(argv[2], "x");
  long y = parse_long(argv[3], "y");
  long delta_x = parse_long(argv[4], "delta_x");
  long delta_y = parse_long(argv[5], "delta_y");
  long count = parse_long(argv[6], "count");
  long interval_ms = parse_long(argv[7], "interval_ms");
  if (count < 1 || count > 1000 || interval_ms < 0 || interval_ms > 1000) {
    fprintf(stderr, "count or interval_ms out of range\n");
    exit(2);
  }

  CGEventSourceRef source =
      CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
  if (source == NULL) {
    fprintf(stderr, "could not create event source\n");
    exit(1);
  }
  for (long index = 0; index < count; index++) {
    CGPoint point = CGPointMake((CGFloat)x, (CGFloat)y);
    CGEventRef event = CGEventCreateMouseEvent(source, kCGEventMouseMoved,
                                               point, kCGMouseButtonLeft);
    if (event == NULL) {
      fprintf(stderr, "could not create mouse event\n");
      CFRelease(source);
      exit(1);
    }
    CGEventSetIntegerValueField(event, kCGMouseEventDeltaX, delta_x);
    CGEventSetIntegerValueField(event, kCGMouseEventDeltaY, delta_y);
    CGEventPost(kCGHIDEventTap, event);
    CFRelease(event);
    if (interval_ms > 0) {
      usleep((useconds_t)interval_ms * 1000);
    }
  }
  CFRelease(source);
  print_position();
}

static CGMouseButton parse_button(const char *value) {
  if (strcmp(value, "left") == 0) return kCGMouseButtonLeft;
  if (strcmp(value, "right") == 0) return kCGMouseButtonRight;
  if (strcmp(value, "center") == 0) return kCGMouseButtonCenter;
  fprintf(stderr, "invalid button: %s\n", value);
  exit(2);
}

static void post_button(const char *button_name, const char *state) {
  CGMouseButton button = parse_button(button_name);
  bool pressed = strcmp(state, "down") == 0;
  if (!pressed && strcmp(state, "up") != 0) {
    fprintf(stderr, "button state must be down or up\n");
    exit(2);
  }
  CGEventRef current = CGEventCreate(NULL);
  CGPoint point = CGEventGetLocation(current);
  CFRelease(current);
  CGEventType type;
  if (button == kCGMouseButtonLeft) type = pressed ? kCGEventLeftMouseDown : kCGEventLeftMouseUp;
  else if (button == kCGMouseButtonRight) type = pressed ? kCGEventRightMouseDown : kCGEventRightMouseUp;
  else type = pressed ? kCGEventOtherMouseDown : kCGEventOtherMouseUp;
  CGEventRef event = CGEventCreateMouseEvent(NULL, type, point, button);
  CGEventPost(kCGHIDEventTap, event);
  CFRelease(event);
}

static void post_key(long key_code, const char *state) {
  bool pressed = strcmp(state, "down") == 0;
  if (!pressed && strcmp(state, "up") != 0) {
    fprintf(stderr, "key state must be down or up\n");
    exit(2);
  }
  CGEventSourceRef source =
      CGEventSourceCreate(kCGEventSourceStateHIDSystemState);
  if (source == NULL) {
    fprintf(stderr, "could not create keyboard event source\n");
    exit(1);
  }
  CGEventRef event = CGEventCreateKeyboardEvent(source, (CGKeyCode)key_code, pressed);
  if (event == NULL) {
    fprintf(stderr, "could not create keyboard event\n");
    CFRelease(source);
    exit(1);
  }
  CGEventSetIntegerValueField(event, kCGEventSourceUserData, 0);
  CGEventPost(kCGHIDEventTap, event);
  CFRelease(event);
  CFRelease(source);
}

int main(int argc, char **argv) {
  if (argc == 2 && strcmp(argv[1], "position") == 0) {
    print_position();
    return 0;
  }
  if (argc == 2 && strcmp(argv[1], "displays") == 0) {
    print_displays();
    return 0;
  }
  if (argc >= 2 && strcmp(argv[1], "move") == 0) {
    move_pointer(argc, argv);
    return 0;
  }
  if (argc == 4 && strcmp(argv[1], "button") == 0) {
    post_button(argv[2], argv[3]);
    return 0;
  }
  if (argc == 3 && strcmp(argv[1], "button-state") == 0) {
    printf("%d\n", CGEventSourceButtonState(kCGEventSourceStateCombinedSessionState,
                                             parse_button(argv[2])));
    return 0;
  }
  if (argc == 4 && strcmp(argv[1], "key") == 0) {
    post_key(parse_long(argv[2], "key_code"), argv[3]);
    return 0;
  }
  if (argc == 3 && strcmp(argv[1], "key-state") == 0) {
    long key_code = parse_long(argv[2], "key_code");
    printf("%d\n", CGEventSourceKeyState(kCGEventSourceStateCombinedSessionState,
                                          (CGKeyCode)key_code));
    return 0;
  }
  fprintf(stderr,
          "usage: macos_pointer_probe position|displays|move|button|button-state|key|key-state ...\n");
  return 2;
}
