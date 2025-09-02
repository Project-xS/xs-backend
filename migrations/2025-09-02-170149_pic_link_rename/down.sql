-- Revert column rename `has_pic` back to `pic_link`
ALTER TABLE public.menu_items
    RENAME COLUMN has_pic TO pic_link;

ALTER TABLE public.canteens
    RENAME COLUMN has_pic TO pic_link;
